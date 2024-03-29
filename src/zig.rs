use crate::config::config_path;
use crate::config::Config;

use fs_err::OpenOptions;
use std::ffi::{OsStr, OsString};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::{atomic::AtomicBool, Arc};

const MIN_MAIN_CPP: &str = r#"
int main(int argc, char** argv) {
    return 0;
    (void)argc;
    (void)argv;
}
"#;

enum OutType {
    Stdout,
    Stderr,
}

fn trace_child_stream<R>(
    out: OutType,
    mut stream: R,
) -> std::thread::JoinHandle<anyhow::Result<Vec<u8>>>
where
    R: Read + Send + 'static,
{
    std::thread::spawn(move || {
        let mut res = Vec::new();
        let mut buf = [0u8; 1024];
        loop {
            match stream.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    match out {
                        OutType::Stdout => {
                            let mut out = std::io::stdout().lock();
                            out.write_all(&buf[..n])?;
                        }
                        OutType::Stderr => {
                            let mut out = std::io::stderr().lock();
                            out.write_all(&buf[..n])?;
                        }
                    };
                    res.extend_from_slice(&buf[..n]);
                }
                Err(e) => {
                    eprintln!("Error reading child stream: {}", e);
                    break;
                }
            }
        }
        Ok(res)
    })
}

#[allow(dead_code)]
fn stdio_pipe<W>(
    mut stream: W,
    is_done: Arc<AtomicBool>,
) -> std::thread::JoinHandle<anyhow::Result<Vec<u8>>>
where
    W: Write + Send + 'static,
{
    std::thread::spawn(move || {
        let mut res = Vec::new();
        let mut buf = [0u8; 1024];
        while !is_done.load(std::sync::atomic::Ordering::Relaxed) {
            match std::io::stdin().read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    stream.write_all(&buf[..n])?;
                    res.extend_from_slice(&buf[..n]);
                }
                Err(e) => {
                    eprintln!("Error reading stdin: {}", e);
                    break;
                }
            }
        }
        Ok(res)
    })
}

struct TraceData<'td> {
    args: &'td Vec<String>,
    cmd: &'td str,
    cd: &'td str,
    path: &'td str,
    exit_code: i32,
    stdin: &'td Vec<u8>,
    stdout: &'td Vec<u8>,
    stderr: &'td Vec<u8>,
}

fn save_trace(log_file: &std::path::Path, td: TraceData<'_>) -> anyhow::Result<()> {
    let mut trace_file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(log_file)?;

    trace_file.write_all(b"---\nARGS:")?;
    trace_file.write_all(td.args.join(" ").as_bytes())?;
    trace_file.write_all(b"\nCMD:")?;
    trace_file.write_all(td.cmd.as_bytes())?;
    trace_file.write_all(b"\nCWD:")?;
    trace_file.write_all(td.cd.as_bytes())?;
    trace_file.write_all(b"\nPATH:")?;
    trace_file.write_all(td.path.as_bytes())?;
    trace_file.write_all(b"\nEXITCODE:")?;
    trace_file.write_all(td.exit_code.to_string().as_bytes())?;
    trace_file.write_all(b"\nSTDIN:\n")?;
    trace_file.write_all(td.stdin)?;
    trace_file.write_all(b"\nSTDOUT:\n")?;
    trace_file.write_all(td.stdout)?;
    trace_file.write_all(b"\nSTDERR:\n")?;
    trace_file.write_all(td.stderr)?;
    trace_file.write_all(b"\n***\n")?;
    Ok(())
}

fn tmpname(prefix: &OsStr, suffix: &OsStr, rand_len: usize) -> OsString {
    let mut buf = OsString::with_capacity(prefix.len() + suffix.len() + rand_len);
    buf.push(prefix);
    let mut char_buf = [0u8; 4];
    for c in std::iter::repeat_with(fastrand::alphanumeric).take(rand_len) {
        buf.push(c.encode_utf8(&mut char_buf));
    }
    buf.push(suffix);
    buf
}

fn make_tmp_main_cpp() -> anyhow::Result<(String, String)> {
    let tmp = std::env::temp_dir();
    let tmp_main = tmp.join(tmpname(OsStr::new("zig-cc-"), OsStr::new(".cpp"), 5));
    let tmp_obj = tmp.join(tmpname(OsStr::new("zig-cc-"), OsStr::new(".obj"), 5));
    OpenOptions::new()
        .write(true)
        .create(true)
        .open(&tmp_main)?
        .write_all(MIN_MAIN_CPP.as_bytes())?;

    Ok((
        tmp_main.display().to_string(),
        tmp_obj.display().to_string(),
    ))
}

fn run_zig(
    zig: &str,
    tool: &str,
    args0: &[String],
    args1: &[String],
    trace: bool,
) -> anyhow::Result<i32> {
    let print_search_dir = args0.contains(&"-print-search-dirs".to_string())
        || args1.contains(&"-print-search-dirs".to_string());
    let print_zig_version =
        args0.contains(&"-version".to_string()) || args1.contains(&"-version".to_string());

    let dir = std::env::current_exe()?
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Can't get parent directory."))?
        .to_owned();
    let cd = match std::env::current_dir() {
        Ok(v) => v.display().to_string(),
        Err(e) => {
            format!("Error getting current directory: {}", e)
        }
    };

    let cwd = if print_search_dir
        || (cd.contains("QtCreator") && cd.contains("bin"))
        || print_zig_version
    {
        dir.display().to_string()
    } else {
        cd
    };

    let track_stdin = false;

    let (tmp_main, tmp_obj) = make_tmp_main_cpp()?;

    defer_lite::defer!({
        let _ = std::fs::remove_file(&tmp_main);
        let _ = std::fs::remove_file(&tmp_obj);
    });

    let mut args: Vec<String> = Vec::new();

    if tool == "cc" || tool == "c++" {
        args.extend_from_slice(args0);
        for arg in args1 {
            if arg == "-" || arg == "nul" {
                if args.iter().any(|x| x == "-dM") {
                    args.push(tmp_main.clone())
                } else {
                    args.push("-c".to_string());
                    args.push(tmp_main.clone());
                    args.push("-o".to_string());
                    args.push(tmp_obj.clone());
                }
            } else {
                args.push(arg.clone());
            }
        }
    } else {
        args.extend_from_slice(args0);
        args.extend_from_slice(args1);
    }

    let cmd = format!("{} {} {}", zig, tool, args.join(" "));
    // print!("{}", cmd);

    let path = std::env::var("PATH").unwrap_or_else(|e| format!("Error:{}", e));
    let zig_dir = std::path::Path::new(zig)
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .display()
        .to_string();
    let path_sep = if cfg!(windows) { ";" } else { ":" };
    let zig_done = Arc::new(AtomicBool::new(false));
    let mut run_cmd = Command::new(zig)
        .env("PATH", format!("{}{}{}", zig_dir, path_sep, path))
        .current_dir(&cwd)
        .arg(tool)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(if track_stdin {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .spawn()?;

    let out = trace_child_stream(
        OutType::Stdout,
        run_cmd
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Can't take stdout for {}", tool))?,
    );
    let err = trace_child_stream(
        OutType::Stderr,
        run_cmd
            .stderr
            .take()
            .ok_or_else(|| anyhow::anyhow!("Can't take stderr for {}", tool))?,
    );
    let stdin = if track_stdin {
        stdio_pipe(
            run_cmd
                .stdin
                .take()
                .ok_or_else(|| anyhow::anyhow!("Can't take stdin for {}", tool))?,
            zig_done.clone(),
        )
    } else {
        std::thread::spawn(|| Ok(Vec::new()))
    };

    match run_cmd.wait()?.code() {
        Some(n) => {
            zig_done.store(true, std::sync::atomic::Ordering::Relaxed);
            if trace {
                let out = match out.join() {
                    Ok(Ok(v)) => v,
                    Ok(Err(e)) => {
                        eprintln!("Error reading child stream: {}", e);
                        Vec::new()
                    }
                    Err(_e) => {
                        eprintln!("Error reading stdout for {}", tool);
                        Vec::new()
                    }
                };
                let err = match err.join() {
                    Ok(Ok(v)) => v,
                    Ok(Err(e)) => {
                        eprintln!("Error reading child stream: {}", e);
                        Vec::new()
                    }
                    Err(_e) => {
                        eprintln!("Error reading stderr for {}", tool);
                        Vec::new()
                    }
                };
                let stdin = match stdin.join() {
                    Ok(Ok(v)) => v,
                    Ok(Err(e)) => {
                        eprintln!("Error reading stdin {}", e);
                        Vec::new()
                    }
                    Err(_e) => {
                        eprintln!("Error reading stdin for {}", tool);
                        Vec::new()
                    }
                };

                let args = std::env::args().collect::<Vec<_>>();
                let td = TraceData {
                    args: &args,
                    cmd: &cmd,
                    cd: &cwd,
                    path: &path,
                    exit_code: n,
                    stdin: &stdin,
                    stdout: &out,
                    stderr: &err,
                };
                save_trace(&crate::config::tool_trace_file(tool)?, td)?;
            };
            Ok(n)
        }
        None => anyhow::bail!("Fail to run`{}` with unknown code.", cmd),
    }
}

#[allow(dead_code)]
pub(crate) fn zig_c() -> anyhow::Result<i32> {
    let config = Config::from_file(config_path()?)?;

    let args: Vec<String> = std::env::args().skip(1).collect();

    run_zig(&config.zig, "cc", &config.c_options, &args, config.trace)
}

#[allow(dead_code)]
pub(crate) fn zig_cpp() -> anyhow::Result<i32> {
    let config = Config::from_file(config_path()?)?;
    let args: Vec<String> = std::env::args().skip(1).collect();

    run_zig(&config.zig, "c++", &config.cpp_options, &args, config.trace)
}

#[allow(dead_code)]
pub(crate) fn zig_tool<S: AsRef<str>>(tool: S) -> anyhow::Result<i32> {
    let config = Config::from_file(config_path()?)?;
    let args: Vec<String> = std::env::args().skip(1).collect();

    run_zig(
        &config.zig,
        tool.as_ref(),
        &config.tools_options,
        &args,
        config.trace,
    )
}
