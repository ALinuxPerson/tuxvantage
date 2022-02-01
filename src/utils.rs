use itertools::Itertools;
use serde::{de, Deserialize, Serialize};
use std::{fmt, thread, io};
use std::ops::Not;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use anyhow::Context;
use crossbeam::channel::Receiver;
use owo_colors::OwoColorize;

pub fn dedup_error_chain_for_humans(error: anyhow::Error) -> String {
    error.chain().map(ToString::to_string).unique().join(": ")
}

pub fn not<T: Not>(value: T) -> T::Output {
    !value
}

#[derive(Serialize, Default, Copy, Clone)]
pub struct FromStrDeserializer<T>(pub T)
where
    T: FromStr,
    T::Err: fmt::Display;

impl<'de, T> Deserialize<'de> for FromStrDeserializer<T>
where
    T: FromStr,
    T::Err: fmt::Display,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse::<T>()
            .map_err(de::Error::custom)
            .map(Self)
    }
}

impl<T> FromStr for FromStrDeserializer<T>
where
    T: FromStr,
    T::Err: fmt::Display,
{
    type Err = T::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<T>().map(Self)
    }
}

impl<T> fmt::Display for FromStrDeserializer<T>
where
    T: fmt::Display,
    T: FromStr,
    T::Err: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Deserialize, Default, Copy, Clone)]
pub struct DisplaySerializer<T: fmt::Display>(pub T);

impl<T: fmt::Display> Serialize for DisplaySerializer<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<T: fmt::Display> fmt::Display for DisplaySerializer<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> FromStr for DisplaySerializer<T>
where
    T: fmt::Display,
    T: FromStr,
    T::Err: fmt::Display,
{
    type Err = T::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<T>().map(Self)
    }
}

pub fn sleep(duration: Duration) -> Receiver<()> {
    let (sender, receiver) = crossbeam::channel::bounded(1);

    thread::spawn(move || {
        thread::sleep(duration);
        sender.send(()).expect("failed to send sleep event");
    });

    receiver
}

pub fn is_systemd() -> anyhow::Result<bool> {
    match Path::new("/run/systemd/system").metadata() {
        Ok(_) => Ok(true),
        Err(error) if matches!(error.kind(), io::ErrorKind::NotFound) => Ok(false),
        Err(error) => Err(error)
            .with_context(|| format!("failed to check if the current init system is {}", "systemd".bold()))
    }
}
