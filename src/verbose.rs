use std::sync::atomic::{AtomicBool, Ordering};

static VERBOSE: AtomicBool = AtomicBool::new(false);

pub fn get() -> bool {
    VERBOSE.load(Ordering::SeqCst)
}

pub fn set(verbose: bool) {
    VERBOSE.store(verbose, Ordering::SeqCst);
}

pub fn enabled() -> bool {
    get()
}

pub fn disabled() -> bool {
    !get()
}
