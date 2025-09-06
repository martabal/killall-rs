use std::{
    fs,
    io::{self, BufRead, BufReader},
    os::unix::ffi::OsStrExt,
    process,
};

use clap::{
    Parser,
    builder::{Styles, styling::AnsiColor},
};
use nix::{
    sys::signal::{Signal, kill},
    unistd::Pid,
};

const MAX_NAMES: usize = std::mem::size_of::<usize>() * 8;

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().bold())
    .usage(AnsiColor::Green.on_default().bold())
    .literal(AnsiColor::Cyan.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default());

#[derive(Parser, Debug)]
#[command(styles = STYLES)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// process name to kill
    #[arg(required_unless_present = "list")]
    process_names: Vec<String>,

    /// list all known signal names
    #[arg(short = 'l', long)]
    list: bool,

    /// Send this signal instead of SIGTERM
    #[arg(short = 's', long)]
    signal: Option<String>,
}

fn list_pids_by_comm(target_name: &str) -> io::Result<Vec<u32>> {
    let target_bytes = target_name.as_bytes();

    #[cfg(feature = "rayon")]
    use rayon::prelude::*;

    #[cfg(feature = "rayon")]
    let results: Vec<u32> = fs::read_dir("/proc")?
        .par_bridge()
        .filter_map(|e| e.ok().and_then(|entry| check_entry(&entry, target_bytes)))
        .collect();

    #[cfg(not(feature = "rayon"))]
    let results: Vec<u32> = fs::read_dir("/proc")?
        .filter_map(|e| e.ok().and_then(|entry| check_entry(&entry, target_bytes)))
        .collect();

    Ok(results)
}

fn check_entry(entry: &fs::DirEntry, target_bytes: &[u8]) -> Option<u32> {
    let file_name = entry.file_name();
    let file_name_bytes = file_name.as_bytes();

    if !file_name_bytes.iter().all(|&b| b.is_ascii_digit()) {
        return None;
    }

    let pid = parse_pid_from_bytes(file_name_bytes)?;
    let comm_path = entry.path().join("comm");

    let file = fs::File::open(comm_path).ok()?;
    let mut reader = BufReader::new(file);
    let mut comm_buf = Vec::with_capacity(16);

    if reader.read_until(b'\n', &mut comm_buf).is_ok() {
        if comm_buf.last() == Some(&b'\n') {
            comm_buf.pop();
        }
        if comm_buf == target_bytes {
            return Some(pid);
        }
    }

    None
}

#[inline]
fn parse_pid_from_bytes(bytes: &[u8]) -> Option<u32> {
    if bytes.is_empty() || bytes.len() > 10 {
        return None;
    }

    let mut result = 0u32;
    for &byte in bytes {
        if !byte.is_ascii_digit() {
            return None;
        }
        result = result.checked_mul(10)?.checked_add((byte - b'0') as u32)?;
    }

    if result == 0 { None } else { Some(result) }
}

static SIGNALS: &[(&str, Signal)] = &[
    ("INT", Signal::SIGINT),
    ("TERM", Signal::SIGTERM),
    ("KILL", Signal::SIGKILL),
    ("HUP", Signal::SIGHUP),
    ("QUIT", Signal::SIGQUIT),
    ("USR1", Signal::SIGUSR1),
    ("USR2", Signal::SIGUSR2),
    ("ALRM", Signal::SIGALRM),
    ("CONT", Signal::SIGCONT),
    ("STOP", Signal::SIGSTOP),
    ("TSTP", Signal::SIGTSTP),
    ("CHLD", Signal::SIGCHLD),
    ("PIPE", Signal::SIGPIPE),
    ("SEGV", Signal::SIGSEGV),
    ("ABRT", Signal::SIGABRT),
    ("ILL", Signal::SIGILL),
    ("TRAP", Signal::SIGTRAP),
    ("BUS", Signal::SIGBUS),
    ("FPE", Signal::SIGFPE),
    ("TTIN", Signal::SIGTTIN),
    ("TTOU", Signal::SIGTTOU),
    ("URG", Signal::SIGURG),
    ("XCPU", Signal::SIGXCPU),
    ("XFSZ", Signal::SIGXFSZ),
    ("VTALRM", Signal::SIGVTALRM),
    ("PROF", Signal::SIGPROF),
    ("WINCH", Signal::SIGWINCH),
    ("IO", Signal::SIGIO),
    ("PWR", Signal::SIGPWR),
    ("SYS", Signal::SIGSYS),
];

fn parse_signal(name: &str) -> Option<Signal> {
    let upper = name.to_uppercase();

    SIGNALS
        .iter()
        .find(|(sig_name, _)| *sig_name == upper.as_str())
        .map(|(_, signal)| *signal)
}

fn list_signals() -> String {
    SIGNALS
        .iter()
        .map(|(name, _)| *name)
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() {
    let args = Args::parse();

    if args.list {
        println!("{}", list_signals());
        return;
    }

    if args.process_names.len() > MAX_NAMES {
        eprintln!(
            "{}: Maximum number of names is {} and you gave {}",
            env!("CARGO_PKG_NAME"),
            MAX_NAMES,
            args.process_names.len(),
        );
        process::exit(1);
    }

    let sig = match args.signal.as_deref() {
        Some(name) => match parse_signal(name) {
            Some(s) => s,
            None => {
                eprintln!("{name}: unknown signal");
                process::exit(1);
            }
        },
        None => Signal::SIGTERM,
    };

    for process_name in args.process_names.iter() {
        let pids = match list_pids_by_comm(process_name) {
            Ok(pids) => pids,
            Err(e) => {
                eprintln!("Error: {}", e);
                continue;
            }
        };

        if pids.is_empty() {
            eprintln!("{}: no process found", process_name);
            continue;
        }

        for pid in pids {
            if let Err(err) = kill(Pid::from_raw(pid as i32), sig) {
                eprintln!("Failed to send signal to {}: {}", pid, err);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn setup_proc_dir() -> PathBuf {
        let tmp_dir = PathBuf::from("tests/tmp_proc");
        if tmp_dir.exists() {
            fs::remove_dir_all(&tmp_dir).unwrap();
        }
        fs::create_dir_all(&tmp_dir).unwrap();
        tmp_dir
    }

    fn cleanup_proc_dir(dir: &PathBuf) {
        if dir.exists() {
            fs::remove_dir_all(dir).unwrap();
        }
    }

    #[test]
    fn test_parse_pid_from_bytes() {
        assert_eq!(parse_pid_from_bytes(b"1234"), Some(1234));
        assert_eq!(parse_pid_from_bytes(b"0"), None);
        assert_eq!(parse_pid_from_bytes(b"abc"), None);
        assert_eq!(parse_pid_from_bytes(b""), None);
        assert_eq!(parse_pid_from_bytes(b"4294967295"), Some(4294967295));
        assert_eq!(parse_pid_from_bytes(b"4294967296"), None);
    }

    #[test]
    fn test_list_pids_by_comm_none() {
        let tmp_dir = setup_proc_dir();

        let result = list_pids_by_comm_in_dir("nonexistent", &tmp_dir).unwrap();
        assert!(result.is_empty());

        cleanup_proc_dir(&tmp_dir);
    }

    fn list_pids_by_comm_in_dir(
        target_name: &str,
        dir: &std::path::Path,
    ) -> std::io::Result<Vec<u32>> {
        let mut pids = Vec::new();
        let target_bytes = target_name.as_bytes();
        let mut comm_buf = Vec::with_capacity(16);

        for entry in fs::read_dir(dir)? {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let file_name = entry.file_name();
            let file_name_bytes = file_name.as_bytes();

            if !file_name_bytes.iter().all(|&b| b.is_ascii_digit()) {
                continue;
            }

            let pid = match parse_pid_from_bytes(file_name_bytes) {
                Some(p) => p,
                None => continue,
            };

            let comm_path = entry.path().join("comm");

            match fs::File::open(&comm_path) {
                Ok(file) => {
                    let mut reader = BufReader::new(file);
                    comm_buf.clear();

                    if reader.read_until(b'\n', &mut comm_buf).is_ok() {
                        if comm_buf.last() == Some(&b'\n') {
                            comm_buf.pop();
                        }

                        if comm_buf == target_bytes {
                            pids.push(pid);
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        Ok(pids)
    }
}
