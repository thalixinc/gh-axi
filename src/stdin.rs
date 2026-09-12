//! Stdin helpers, mirroring `src/stdin.ts`.

use std::io::{IsTerminal, Read};

/// Read all of this process's stdin as a UTF-8 string.
pub fn read_stdin() -> String {
    let mut data = String::new();
    let _ = std::io::stdin().lock().read_to_string(&mut data);
    data
}

/// Whether stdin is an interactive terminal (no piped input available).
pub fn is_stdin_tty() -> bool {
    std::io::stdin().is_terminal()
}
