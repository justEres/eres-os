use std::env;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let script = format!("{}/scripts/run_qemu.sh", env!("CARGO_MANIFEST_DIR"));
    let args: Vec<String> = env::args().skip(1).collect();

    let status = match Command::new(&script).args(args).status() {
        Ok(status) => status,
        Err(err) => {
            eprintln!("Failed to execute {}: {}", script, err);
            return ExitCode::FAILURE;
        }
    };

    if status.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
