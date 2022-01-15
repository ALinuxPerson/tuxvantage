use crate::log::Level;
use parking_lot::RwLock;

static NO_PROLOGUE_FOR: RwLock<Option<Level>> = parking_lot::const_rwlock(None);

pub fn r#for(level: Level) {
    *NO_PROLOGUE_FOR.write() = Some(level);
}

pub fn reenable() {
    *NO_PROLOGUE_FOR.write() = None;
}

pub fn for_what() -> Option<Level> {
    *NO_PROLOGUE_FOR.read()
}

pub struct Guard {
    _priv: (),
}

impl Guard {
    pub fn r#for(level: Level) -> Self {
        r#for(level);
        Self { _priv: () }
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        reenable()
    }
}

pub fn guard_for(level: Level) -> Guard {
    Guard::r#for(level)
}
