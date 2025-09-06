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

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().bold())
    .usage(AnsiColor::Green.on_default().bold())
    .literal(AnsiColor::Cyan.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default());

#[derive(Parser, Debug)]
#[command(styles = STYLES)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Process name to kill
    process_name: String,
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

fn main() {
    let args = Args::parse();

    let pids = match list_pids_by_comm(&args.process_name) {
        Ok(pids) => pids,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    if pids.is_empty() {
        eprintln!("{}: no process found", args.process_name);
        process::exit(1);
    }

    for pid in pids {
        if let Err(err) = kill(Pid::from_raw(pid as i32), Signal::SIGTERM) {
            eprintln!("Failed to kill {}: {}", pid, err);
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
