use ideapad::context::Context as IdeapadContext;
use once_cell::sync::OnceCell;
use try_drop::drop_strategies::broadcast::NeedsReceivers;
use try_drop::drop_strategies::{BroadcastDropStrategy, PanicDropStrategy};

pub type Context = IdeapadContext<BroadcastDropStrategy<NeedsReceivers>, PanicDropStrategy>;

static CONTEXT: OnceCell<Context> = OnceCell::new();

pub fn initialize(context: Context) {
    if CONTEXT.set(context).is_err() {
        panic!("context is already initialized")
    }
}

pub fn get() -> &'static Context {
    CONTEXT.get().expect("context is not initialized")
}
