//! gh-axi — AXI-compliant wrapper around the GitHub CLI.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let outcome = gh_axi::cli::run(&args);
    use std::io::Write;
    let mut stdout = std::io::stdout().lock();
    let _ = stdout.write_all(outcome.output.as_bytes());
    let _ = stdout.flush();
    std::process::exit(outcome.exit_code);
}
