use std::process;

use clap::Parser;
use nix::{
    sys::signal::{Signal, kill},
    unistd::Pid,
};

use faulx::{
    cli::{FaulxArgs, MAX_NAMES},
    processes::list_pids_by_comm,
    signals::{list_signals, parse_signal},
};

fn main() {
    let args = FaulxArgs::parse();

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
            process::exit(1);
        }

        for pid in pids {
            if let Err(err) = kill(Pid::from_raw(pid), sig) {
                eprintln!("Failed to send signal to {}: {}", pid, err);
            }
        }
    }
}
