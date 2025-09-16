use std::{process, sync::atomic::Ordering};

use clap::Parser;
use nix::{
    sys::signal::{Signal, kill},
    unistd::Pid,
};

use faulx::{
    cli::{FaulxArgs, MAX_NAMES},
    macros::QUIET,
    processes::list_pids,
    qprintln,
    signals::{list_signals, parse_signal},
};

fn main() {
    let args = FaulxArgs::parse();

    QUIET.store(args.quiet, Ordering::Relaxed);

    if args.list {
        println!("{}", list_signals());
        return;
    }

    if args.process_names.len() > MAX_NAMES {
        qprintln!(
            "{}: Maximum number of names is {} and you gave {}",
            env!("CARGO_PKG_NAME"),
            MAX_NAMES,
            args.process_names.len(),
        );
        process::exit(1);
    }

    let sig = args.signal.as_deref().map_or(Signal::SIGTERM, |name| {
        parse_signal(name).map_or_else(
            || {
                qprintln!("{name}: unknown signal");
                process::exit(1);
            },
            |s| s,
        )
    });

    for process_name in &args.process_names {
        let pids = match list_pids(process_name, args.process_group) {
            Ok(pids) => pids,
            Err(e) => {
                qprintln!("Error: {e}");
                continue;
            }
        };

        if pids.is_empty() {
            qprintln!("{process_name}: no process found");
            process::exit(1);
        }

        for pid in pids {
            if let Err(err) = kill(Pid::from_raw(pid), sig) {
                qprintln!("Failed to send signal to {pid}: {err}");
            } else if args.verbose {
                println!("Killed {process_name}({pid}) with signal {}", sig as i32);
            }
        }
    }
}
