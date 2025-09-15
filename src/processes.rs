use std::{fs, io, os::unix::ffi::OsStrExt};

#[cfg(feature = "rayon")]
use rayon::prelude::*;

const PROC: &str = "/proc";

pub fn list_pids_by_comm(target_name: &str) -> io::Result<Vec<i32>> {
    let target_bytes = target_name.as_bytes();

    let entries = fs::read_dir(PROC)?;

    #[cfg(feature = "rayon")]
    let iter = entries.par_bridge();

    #[cfg(not(feature = "rayon"))]
    let iter = entries.into_iter();

    Ok(iter
        .filter_map(|e| e.ok().and_then(|entry| check_entry(&entry, target_bytes)))
        .collect())
}

fn check_entry(entry: &fs::DirEntry, target_bytes: &[u8]) -> Option<i32> {
    let pid = parse_pid_from_bytes(entry.file_name().as_bytes())?;

    let comm_path = format!("{PROC}/{pid}/comm");

    let mut buf = [0u8; 64];
    let len = fs::File::open(comm_path)
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
    fn test_list_pids_by_comm_no_match() {
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
