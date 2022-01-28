use std::fs::{DirEntry, ReadDir};
use std::path::PathBuf;
use std::{fs, io};

use anyhow::Context;
use ideapad::Profile;
use owo_colors::OwoColorize;

use crate::project_paths;

pub struct Profiles {
    entries: ReadDir,
}

impl Profiles {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            entries: project_paths::profiles_dir()
                .read_dir()
                .context("failed to get entries of the profile directory")?,
        })
    }
}

#[derive(Clone)]
pub struct ExternalProfile {
    pub profile: Profile,
    pub path: PathBuf,
}

impl Iterator for Profiles {
    type Item = anyhow::Result<ExternalProfile>;

    fn next(&mut self) -> Option<Self::Item> {
        fn inner(entry: io::Result<DirEntry>) -> anyhow::Result<ExternalProfile> {
            let path = entry
                .context("failed to get the next entry of the profile directory")?
                .path();
            let contents = fs::read_to_string(&path).with_context(|| {
                format!("failed to read contents of profile {}", path.display().bold())
            })?;

            let profile = serde_json::from_str(&contents).with_context(|| {
                format!(
                    "failed to deserialize contents of profile {}",
                    path.display().bold()
                )
            })?;

            Ok(ExternalProfile { profile, path })
        }

        Some(inner(self.entries.next()?))
    }
}
