use std::process::ExitCode;

fn main() -> ExitCode {
    // TODO: use lib.rs APIs
    println!("add() returns {}", tunnel::add(1, 1));
    ExitCode::SUCCESS
}
