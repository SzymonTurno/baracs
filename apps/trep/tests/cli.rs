use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::thread;

fn run(pattern: &str, input: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_trep"))
        .arg(pattern)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn matching_lines_are_printed() {
    let out = run("hello", "hello world\nfoo bar\nhello again\n");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "hello world\nhello again\n");
}

#[test]
fn non_matching_lines_are_not_printed() {
    let out = run("foo", "hello world\nfoo bar\n");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "foo bar\n");
}

#[test]
fn exit_0_on_match() {
    let out = run("foo", "foo\n");
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn exit_1_on_no_match() {
    let out = run("foo", "bar\nbaz\n");
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn exit_1_on_empty_input() {
    let out = run("foo", "");
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn exit_2_on_invalid_pattern() {
    let out = run("[invalid", "anything\n");
    assert_eq!(out.status.code(), Some(2));
    assert!(!out.stderr.is_empty());
}

#[test]
fn exit_2_on_no_args() {
    let out = Command::new(env!("CARGO_BIN_EXE_trep"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(!out.stderr.is_empty());
}

#[test]
fn anchor_start() {
    let out = run("^foo", "foobar\nbarfoo\n");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "foobar\n");
}

#[test]
fn anchor_end() {
    let out = run("foo$", "foobar\nbarfoo\n");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "barfoo\n");
}

#[test]
fn char_class() {
    let out = run("[0-9]+", "abc\n123\ndef456\n");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "123\ndef456\n");
}

#[test]
fn metaclass_digit() {
    let out = run("\\d+", "abc\n123\n");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "123\n");
}

// Regression: extra arguments beyond PATTERN were silently ignored instead of
// rejected — a user familiar with grep would expect file arguments to work.
#[test]
fn exit_2_on_extra_args() {
    let out = Command::new(env!("CARGO_BIN_EXE_trep"))
        .args(["pattern", "extra_arg"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(!out.stderr.is_empty());
}

// Regression: when broken pipe fired before the first writeln! returned,
// `matched` was still false even though a match had been found, causing exit 1
// instead of the correct exit 0.
#[test]
fn broken_pipe_on_first_match_exits_0() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_trep"))
        .arg(".")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    drop(child.stdout.take());

    let mut stderr = child.stderr.take().unwrap();
    let stderr_thread = thread::spawn(move || {
        let mut buf = Vec::new();
        stderr.read_to_end(&mut buf).unwrap();
        buf
    });

    let _ = child.stdin.take().unwrap().write_all("line\n".repeat(10_000).as_bytes());

    let status = child.wait().unwrap();
    let stderr_output = stderr_thread.join().unwrap();

    assert_eq!(status.code(), Some(0), "match was found; exit must be 0 even on broken pipe");
    assert!(stderr_output.is_empty(), "unexpected stderr: {}", String::from_utf8_lossy(&stderr_output));
}
