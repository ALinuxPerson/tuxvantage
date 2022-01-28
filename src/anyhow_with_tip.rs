use std::fmt;
use std::fmt::Display;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Copy, Clone)]
enum NullDisplay {}

impl Display for NullDisplay {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {}
    }
}

#[derive(Debug)]
pub struct Error {
    pub source: anyhow::Error,
    pub tip: Option<String>,
}

pub trait TippingAnyhowResultExt<T>: Sized {
    fn maybe_tip(self, tip: Option<impl Display>) -> Result<T>;
    fn tip(self, tip: impl Display) -> Result<T> {
        self.maybe_tip(Some(tip.to_string()))
    }
    fn with_tip<F, C>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> C,
        C: Display,
    {
        self.maybe_tip(Some(f().to_string()))
    }
    fn no_tip(self) -> Result<T> {
        let tip: Option<NullDisplay> = None;
        self.maybe_tip(tip)
    }
}

impl<T> TippingAnyhowResultExt<T> for anyhow::Result<T> {
    fn maybe_tip(self, tip: Option<impl Display>) -> Result<T> {
        self.map_err(|e| Error {
            source: e,
            tip: tip.map(|tip| tip.to_string()),
        })
    }
}

impl From<anyhow::Error> for Error {
    fn from(error: anyhow::Error) -> Self {
        Self {
            source: error,
            tip: None,
        }
    }
}

impl From<Error> for anyhow::Error {
    fn from(error: Error) -> Self {
        error.source
    }
}
