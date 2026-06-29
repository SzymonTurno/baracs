use std::ffi::CString;
use std::io::{self, BufRead, Write};
use std::process;

use tiny_regex::Regex;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: {} PATTERN < FILE", args[0]);
        process::exit(2);
    }

    let pattern = CString::new(args[1].as_bytes()).unwrap_or_else(|_| {
        eprintln!("{}: pattern contains a NUL byte", args[0]);
        process::exit(2);
    });

    let re = Regex::new(&pattern).unwrap_or_else(|| {
        eprintln!("{}: invalid pattern: {}", args[0], args[1]);
        process::exit(2);
    });

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut matched = false;

    for line in stdin.lock().lines() {
        let line = line.unwrap_or_else(|e| {
            eprintln!("{}: read error: {}", args[0], e);
            process::exit(2);
        });

        // CString::new fails on embedded NUL bytes; skip any that appear.
        let Ok(cline) = CString::new(line.as_bytes()) else {
            continue;
        };

        if re.find_at(&cline, 0).is_some() {
            matched = true;
            if let Err(e) = writeln!(out, "{}", line) {
                if e.kind() == io::ErrorKind::BrokenPipe {
                    process::exit(0);
                }
                eprintln!("{}: write error: {}", args[0], e);
                process::exit(2);
            }
        }
    }

    process::exit(if matched { 0 } else { 1 });
}
