use nix::sys::signal::Signal;

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

#[must_use]
pub fn parse_signal(name: &str) -> Option<Signal> {
    let upper = name.to_uppercase();

    SIGNALS
        .iter()
        .find(|(sig_name, _)| *sig_name == upper.as_str())
        .map(|(_, signal)| *signal)
}

#[must_use]
pub fn list_signals() -> String {
    SIGNALS
        .iter()
        .map(|(name, _)| *name)
        .collect::<Vec<_>>()
        .join(" ")
}
