use crate::anyhow_with_tip;
use itertools::Itertools;
use serde::Serialize;

#[derive(Serialize)]
#[serde(tag = "status", content = "contents")]
pub enum Machine<S: Serialize> {
    Success(S),
    Failure {
        chain: Vec<String>,
        tip: Option<String>,
    },
}

impl<S: Serialize> Machine<S> {
    pub fn success(value: S) -> Self {
        Self::Success(value)
    }

    pub fn failure(error: impl Into<anyhow_with_tip::Error>) -> Self {
        let error = error.into();
        let chain = error
            .source
            .chain()
            .map(|error| error.to_string())
            .map(strip_ansi_escapes::strip)
            .map(|error| error.expect("failed to strip ansi escapes from error message"))
            .map(String::from_utf8)
            .map(|error| {
                error.expect(
                    "error message contains invalid utf-8 after stripping it of ansi escapes",
                )
            })
            .unique()
            .collect();
        Self::Failure {
            chain,
            tip: error.tip,
        }
    }
}
