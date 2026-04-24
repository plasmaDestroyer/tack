use std::sync::atomic::{AtomicU8, Ordering};

// ANSI color codes
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

// Output mode: 0 = Normal, 1 = Quiet, 2 = Verbose
static OUTPUT_MODE: AtomicU8 = AtomicU8::new(0);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputMode {
    Quiet = 1,
    Normal = 0,
    Verbose = 2,
}

pub fn set_output_mode(mode: OutputMode) {
    OUTPUT_MODE.store(mode as u8, Ordering::SeqCst);
}

pub fn output_mode() -> OutputMode {
    match OUTPUT_MODE.load(Ordering::SeqCst) {
        1 => OutputMode::Quiet,
        2 => OutputMode::Verbose,
        _ => OutputMode::Normal,
    }
}

fn use_color() -> bool {
    std::env::var("NO_COLOR").is_err()
}

pub fn is_quiet() -> bool {
    output_mode() == OutputMode::Quiet
}

pub fn is_verbose() -> bool {
    output_mode() == OutputMode::Verbose
}

/// Print a success message (green) — suppressed in quiet mode
pub fn success(msg: &str) {
    if is_quiet() {
        return;
    }
    if use_color() {
        println!("{BOLD}{GREEN}{msg}{RESET}");
    } else {
        println!("{msg}");
    }
}

/// Print an info message — suppressed in quiet mode
pub fn info(msg: &str) {
    if is_quiet() {
        return;
    }
    println!("{msg}");
}

/// Print a warning message (yellow) — suppressed in quiet mode
pub fn warn(msg: &str) {
    if is_quiet() {
        return;
    }
    if use_color() {
        println!("{YELLOW}{msg}{RESET}");
    } else {
        println!("{msg}");
    }
}

/// Print an error message (red) — always shown, goes to stderr
pub fn error(msg: &str) {
    if use_color() {
        eprintln!("{RED}{msg}{RESET}");
    } else {
        eprintln!("{msg}");
    }
}

/// Print a verbose-only message — only shown in verbose mode
pub fn verbose(msg: &str) {
    if !is_verbose() {
        return;
    }
    if use_color() {
        println!("  {BOLD}{msg}{RESET}");
    } else {
        println!("  {msg}");
    }
}

/// Print a dry-run message (yellow) — always shown
pub fn dry_run(msg: &str) {
    if use_color() {
        println!("{YELLOW}[dry-run]{RESET} {msg}");
    } else {
        println!("[dry-run] {msg}");
    }
}
