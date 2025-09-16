use std::sync::atomic::AtomicBool;

pub static QUIET: AtomicBool = AtomicBool::new(false);

#[macro_export]
macro_rules! qprintln {
    () => {
        if !QUIET.load(Ordering::Relaxed) {
            eprintln!();
        }
    };
    ($($arg:tt)*) => {
        if !QUIET.load(Ordering::Relaxed) {
            eprintln!($($arg)*);
        }
    };
}
