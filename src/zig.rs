use crate::config::config_path;
use crate::config::Config;

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use fs_err::OpenOptions;

struct Out<const IS_STDOUT : bool> ();

impl Out<true> {
    pub(crate) fn out() -> std::io::Stdout {
        std::io::stdout()
    }
}

impl Out<false> {
    pub(crate) fn out() -> std::io::Stderr {
        std::io::stderr()
    }
}

fn trace_child_stream<R, const O: bool>(mut stream: R) -> std::thread::JoinHandle<anyhow::Result<Vec<u8>>>
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
                    if O {
                        let mut out = Out::<true>::out().lock();
                        out.write_all(&buf[..n])?;
                    } else {
                        let mut out = Out::<false>::out().lock();
                        out.write_all(&buf[..n])?;
                    }
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

fn run_zig(
    zig: &str,
    tool: &str,
    args0: &Vec<String>,
    args1: &Vec<String>,
    trace: bool,
) -> anyhow::Result<i32> {
    let cmd = format!("{} {} {} {}", zig, tool, args0.join(" "), args1.join(" "));
    // print!("{}", cmd);

    let mut run_cmd = Command::new(zig)
        .arg(tool)
        .args(args0)
        .args(args1)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let out = trace_child_stream::<_, true>(
        run_cmd
            .stdout
            .take()
            .ok_or(anyhow::anyhow!("Can't take stdout for {}", tool))?,
    );
    let err = trace_child_stream::<_, false>(
        run_cmd
            .stderr
            .take()
            .ok_or(anyhow::anyhow!("Can't take stderr for {}", tool))?,
    );

    match run_cmd.wait()?.code() {
        Some(n) => {
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

                let mut trace_file = OpenOptions::new()
                    .write(true)
                    .append(true)
                    .create(true)
                    .open(crate::config::tool_trace_file(tool)?)?;
                trace_file.write_all(b"---\ncmd:")?;
                trace_file.write_all(cmd.as_bytes())?;
                trace_file.write_all(b"\nexit code:")?;
                trace_file.write_all(n.to_string().as_bytes())?;
                trace_file.write_all(b"\nout:\n")?;
                trace_file.write_all(&out)?;
                trace_file.write_all(b"\nerr:\n")?;
                trace_file.write_all(&err)?;
                trace_file.write_all(b"\n***\n")?;
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

    run_zig(&config.zig, tool.as_ref(), &config.cpp_options, &args, config.trace)
}
