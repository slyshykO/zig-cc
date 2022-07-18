use fs_err::File;
use serde::{Deserialize, Serialize};
use std::io::Read;

const ZIG_CONF_PATH: &str = "zig.toml";

pub fn file_content<P>(file_name: P) -> std::io::Result<String>
where
    P: Into<std::path::PathBuf>,
{
    let mut s = String::new();
    File::open(file_name)
        .and_then(|mut f| f.read_to_string(&mut s))
        .map(|_| s.trim().to_owned())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct Config {
    pub(crate) zig: String,
    pub(crate) c_options: Vec<String>,
    pub(crate) cpp_options: Vec<String>,
    pub(crate) tools_options: Vec<String>,
    #[serde(default)]
    pub(crate) trace: bool,
}

impl Config {
    pub(crate) fn from_file<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Self> {
        let contents = file_content(path.as_ref())?;
        let config: Self = toml::from_str(&contents)?;
        Ok(config)
    }
}

pub fn config_path() -> anyhow::Result<String> {
    let exe = std::env::current_exe()?;
    let dir = exe
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Can't get parent directory from `{:?}`", exe))?;
    let res = dir.join(ZIG_CONF_PATH);
    Ok(res
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Can't convert to String: `{:?}`", res))?
        .into())
}

pub fn tool_trace_file(tool: &str) -> anyhow::Result<std::path::PathBuf> {
    let exe = std::env::current_exe()?;
    let dir = exe
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Can't get parent directory from `{:?}`", exe))?;
    Ok(dir.join(format!("trace-{}.txt", tool)))
}
