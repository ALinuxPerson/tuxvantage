pub mod no_prologue;

use crate::verbose;
use owo_colors::colors::*;
use owo_colors::{Color, OwoColorize};
use std::fmt;

fn log<C, P, M>(prologue: Option<P>, message: M)
where
    C: Color,
    P: fmt::Display,
    M: fmt::Display,
{
    let prologue = match prologue {
        Some(prologue) => prologue.to_string(),
        None => {
            eprintln!("{}", message);
            return;
        }
    };
    let message = message.to_string();
    let mut lines = message.lines();
    let first_line = if let Some(first_line) = lines.next() {
        first_line
    } else {
        return;
    };
    eprintln!("{}{} {}", prologue.fg::<C>().bold(), ":".bold(), first_line);
    let new_prologue = format!("{}{}", " ".repeat(prologue.len()), "|".bold());

    for line in lines {
        eprintln!("{} {}", new_prologue, line);
    }
}

pub fn debug(message: impl fmt::Display, prologue: bool) {
    if verbose::enabled() {
        __debug(message, prologue)
    }
}

macro_rules! log_fn {
    ($($(#[$($meta:meta)*])* $fn_name:ident, $color:ident, $prologue:literal, $level:ident;)*) => {
        $(
        $(#[$($meta)*])*
        pub fn $fn_name(message: impl fmt::Display, prologue: bool) {
            let prologue = if !matches!(no_prologue::for_what(), Some(Level::$level)) && prologue {
                Some($prologue)
            } else {
                None
            };

            log::<$color, _, _>(prologue, message)
        }
        )*
    };
}

#[derive(Copy, Clone)]
pub enum Level {
    Error,
    Warn,
    Info,
    Tip,
    Debug,
}

log_fn! {
    error, Red, "error", Error;
    warn, Yellow, "warn", Warn;
    info, Blue, "info", Info;
    tip, Cyan, "tip", Tip;
    #[doc(hidden)] __debug, Magenta, "debug", Debug;
}
