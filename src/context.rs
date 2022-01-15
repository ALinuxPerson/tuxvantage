use ideapad::context::Context;
use once_cell::sync::OnceCell;

static CONTEXT: OnceCell<Context> = OnceCell::new();

pub fn initialize(context: Context) {
    if CONTEXT.set(context).is_err() {
        panic!("context is already initialized")
    }
}

pub fn get() -> &'static Context {
    CONTEXT.get().expect("context is not initialized")
}