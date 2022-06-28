mod config;
mod zig;

fn main() {
    match zig::zig_tool("dlltool") {
        Ok(0) => (),
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}
