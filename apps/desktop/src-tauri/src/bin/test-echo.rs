//! Test-only stand-in for a child process that speaks a real line protocol: reads stdin one line
//! at a time and writes it straight back to stdout, flushing after every line. Used by
//! `ipc::elevated`'s proxy test instead of a Windows text-filter utility (`findstr`, `more`),
//! whose stdout buffering under a redirected, non-console pipe is not guaranteed to be per-line.
fn main() {
    use std::io::{BufRead, Write};

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines().map_while(Result::ok) {
        if writeln!(stdout, "{line}").is_err() || stdout.flush().is_err() {
            break;
        }
    }
}
