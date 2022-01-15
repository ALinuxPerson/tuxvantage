pub mod profiles;

use anyhow::Context;
use directories::ProjectDirs;
use once_cell::sync::{Lazy, OnceCell};
use profiles::Profiles;
use std::path::{Path, PathBuf};

static PROJECT_DIRS: OnceCell<ProjectDirs> = OnceCell::new();
static PROFILES_DIR: Lazy<PathBuf> = Lazy::new(|| config_dir().join("profiles"));
static TUXVANTAGE_TOML: Lazy<PathBuf> = Lazy::new(|| config_dir().join("tuxvantage.toml"));
const QUALIFIER: &str = "com";
const ORGANIZATION: &str = "ALinuxPerson";
const APPLICATION: &str = "tuxvantage";

pub fn initialize() -> anyhow::Result<()> {
    debug!("initialize project directories, qualifier = '{}', organization = '{}', application = '{}'", QUALIFIER, ORGANIZATION, APPLICATION);
    let project_dirs = ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION)
        .context("failed to get project directories")?;

    let _ = PROJECT_DIRS.set(project_dirs);

    Ok(())
}

pub fn get_dirs() -> &'static ProjectDirs {
    PROJECT_DIRS
        .get()
        .expect("project directories not initialized")
}

pub fn config_dir() -> &'static Path {
    get_dirs().config_dir()
}

pub fn profiles_dir() -> &'static Path {
    PROFILES_DIR.as_ref()
}

pub fn tuxvantage_toml() -> &'static Path {
    TUXVANTAGE_TOML.as_ref()
}

pub fn profiles() -> anyhow::Result<Profiles> {
    Profiles::new()
}
