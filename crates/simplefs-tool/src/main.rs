fn main() {
    if let Err(err) = simplefs_tool::run_from(std::env::args_os()) {
        eprintln!("simplefs-tool: {err}");
        std::process::exit(1);
    }
}
