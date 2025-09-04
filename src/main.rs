use clap::Parser;
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use std::fs::{self, File};
use std::io::{self, Read};
use std::process;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Process name to kill
    name: String,
}

fn list_pids_by_comm(target_name: &str) -> io::Result<Vec<u32>> {
    let mut pids = Vec::new();

    for entry in fs::read_dir("/proc")? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let file_names = entry.file_name();

        let pid_str = match file_names.to_str() {
            Some(s) if s.chars().all(|c| c.is_ascii_digit()) => s,
            _ => continue,
        };

        let comm_path = entry.path().join("comm");
        let mut buf = String::new();
        if let Ok(mut f) = File::open(&comm_path)
            && f.read_to_string(&mut buf).is_ok()
            && buf.trim_end() == target_name
            && let Ok(pid) = pid_str.parse::<u32>()
        {
            pids.push(pid);
        }
    }

    Ok(pids)
}

fn main() {
    let args = Args::parse();
    let pids = match list_pids_by_comm(&args.name) {
        Ok(pids) => pids,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    if pids.is_empty() {
        eprintln!("{}: no process found", args.name);
        process::exit(1);
    } else {
        for pid in pids {
            if let Err(err) = kill(Pid::from_raw(pid as i32), Signal::SIGKILL) {
                eprintln!("Failed to kill {}: {}", pid, err);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
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
    fn test_list_pids_by_comm_single() {
        let tmp_dir = setup_proc_dir();

        let pid_dir = tmp_dir.join("1234");
        fs::create_dir_all(&pid_dir).unwrap();

        let comm_path = pid_dir.join("comm");
        let mut f = File::create(&comm_path).unwrap();
        write!(f, "myprocess\n").unwrap();

        let result = list_pids_by_comm_in_dir("myprocess", &tmp_dir).unwrap();
        assert_eq!(result, vec![1234]);

        cleanup_proc_dir(&tmp_dir);
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

        for entry in fs::read_dir(dir)? {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let file_names = entry.file_name();

            let pid_str = match file_names.to_str() {
                Some(s) if s.chars().all(|c| c.is_ascii_digit()) => s,
                _ => continue,
            };

            let comm_path = entry.path().join("comm");
            let mut buf = String::new();
            if let Ok(mut f) = File::open(&comm_path)
                && f.read_to_string(&mut buf).is_ok()
                && buf.trim_end() == target_name
                && let Ok(pid) = pid_str.parse::<u32>()
            {
                pids.push(pid);
            }
        }

        Ok(pids)
    }
}
