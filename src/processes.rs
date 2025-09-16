use std::{fs, io, os::unix::ffi::OsStrExt};

#[cfg(feature = "rayon")]
use rayon::prelude::*;

const PROC: &str = "/proc";

pub fn list_pids(target_name: &str, use_group: bool) -> io::Result<Vec<i32>> {
    let target_bytes = target_name.as_bytes();
    let entries = fs::read_dir(PROC)?;

    #[cfg(feature = "rayon")]
    let iter = entries.par_bridge();

    #[cfg(not(feature = "rayon"))]
    let iter = entries.into_iter();

    let matching_pids: Vec<i32> = iter
        .filter_map(|e| e.ok().and_then(|entry| check_entry(&entry, target_bytes)))
        .collect();

    if !use_group || matching_pids.is_empty() {
        return Ok(matching_pids);
    }

    let mut groups = Vec::new();
    for &pid in &matching_pids {
        if let Some(pgrp) = check_group(&pid.to_string()) {
            groups.push(pgrp);
        }
    }

    if groups.is_empty() {
        return Ok(matching_pids);
    }

    groups.sort_unstable();
    groups.dedup();

    let entries = fs::read_dir(PROC)?;

    #[cfg(feature = "rayon")]
    let iter = entries.par_bridge();

    #[cfg(not(feature = "rayon"))]
    let iter = entries.into_iter();

    let mut all_pids: Vec<i32> = iter
        .filter_map(std::result::Result::ok)
        .filter_map(|entry| {
            let pid = parse_pid_from_bytes(entry.file_name().as_bytes())?;
            let pgrp = check_group(&pid.to_string())?;
            if groups.binary_search(&pgrp).is_ok() {
                Some(pid)
            } else {
                None
            }
        })
        .collect();

    all_pids.sort_unstable();
    all_pids.dedup();

    Ok(all_pids)
}

fn check_group(pid: &str) -> Option<i32> {
    let stat_path = format!("{PROC}/{pid}/stat");
    let contents = fs::read_to_string(stat_path).ok()?;
    let mut parts = contents.split_whitespace();
    let _pid_field = parts.next()?;
    let _comm_field = parts.next()?;
    let _state_field = parts.next()?;
    let _ppid_field = parts.next()?;
    let pgrp_field = parts.next()?;
    let pgrp: i32 = pgrp_field.parse().ok()?;
    Some(pgrp)
}

fn check_entry(entry: &fs::DirEntry, target_bytes: &[u8]) -> Option<i32> {
    let pid = parse_pid_from_bytes(entry.file_name().as_bytes())?;

    let comm_path = format!("{PROC}/{pid}/comm");
    let mut buf = [0u8; 64];
    let len = fs::File::open(&comm_path)
        .ok()
        .and_then(|mut f| io::Read::read(&mut f, &mut buf).ok())?;
    let name = if len > 0 && buf[len - 1] == b'\n' {
        &buf[..len - 1]
    } else {
        &buf[..len]
    };
    (name == target_bytes).then_some(pid)
}

fn parse_pid_from_bytes(bytes: &[u8]) -> Option<i32> {
    if bytes.is_empty() || bytes.len() > 10 {
        return None;
    }

    let mut result: i32 = 0;
    for &b in bytes {
        if !b.is_ascii_digit() {
            return None;
        }
        result = result.checked_mul(10)?.checked_add((b - b'0').into())?;
    }
    if result == 0 { None } else { Some(result) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_parse_pid_valid() {
        assert_eq!(parse_pid_from_bytes(b"1"), Some(1));
        assert_eq!(parse_pid_from_bytes(b"12345"), Some(12345));
        assert_eq!(parse_pid_from_bytes(b"429496729"), Some(429496729));
    }

    #[test]
    fn test_parse_pid_invalid() {
        assert_eq!(parse_pid_from_bytes(b""), None);
        assert_eq!(parse_pid_from_bytes(b"abc"), None);
        assert_eq!(parse_pid_from_bytes(b"0000"), None);
        assert_eq!(parse_pid_from_bytes(b"18446744073"), None);
    }

    fn unique_test_dir() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("fake_proc_{}", nanos))
    }

    fn setup_fake_proc(tmp: &Path, entries: &[(&str, &str)]) {
        fs::create_dir_all(tmp).unwrap();
        for (pid, comm) in entries {
            let proc_dir = tmp.join(pid);
            fs::create_dir_all(&proc_dir).unwrap();
            let comm_path = proc_dir.join("comm");
            let mut f = File::create(comm_path).unwrap();
            writeln!(f, "{}", comm).unwrap();
        }
    }

    fn cleanup_fake_proc(tmp: &Path) {
        if tmp.exists() {
            fs::remove_dir_all(tmp).unwrap();
        }
    }

    #[test]
    fn test_list_pids_no_match() {
        let tmp = unique_test_dir();
        setup_fake_proc(&tmp, &[("789", "sshd")]);

        let result: Vec<i32> = fs::read_dir(&tmp)
            .unwrap()
            .filter_map(|e| e.ok().and_then(|entry| check_entry(&entry, b"bash")))
            .collect();

        assert!(result.is_empty());

        cleanup_fake_proc(&tmp);
    }
}
