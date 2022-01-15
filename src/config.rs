use anyhow::Context;
use ideapad::{Handler, Profile};
use once_cell::sync::OnceCell;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::ops::{Deref, Not, RangeInclusive};
use std::path::Path;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{env, fmt, fs};
use std::time::Duration;
use battery::{Batteries, Battery};
use tap::{Pipe, Tap};

use crate::project_paths;
use crate::project_paths::profiles::ExternalProfile;
use crate::utils::{DisplaySerializer, FromStrDeserializer};

static EXISTENCE_ENSURED: AtomicBool = AtomicBool::new(false);

pub enum BuiltInProfile {
    Ideapad15IIL05,
    Ideapad15Amd,
}

impl BuiltInProfile {
    pub fn get(&self) -> Profile {
        match self {
            Self::Ideapad15IIL05 => Profile::IDEAPAD_15IIL05,
            Self::Ideapad15Amd => Profile::IDEAPAD_AMD,
        }
    }
}

pub enum PossiblyBuiltInProfile {
    BuiltIn(BuiltInProfile),
    External(Box<ExternalProfile>),
}

impl PossiblyBuiltInProfile {
    pub fn external(profile: ExternalProfile) -> Self {
        Self::External(Box::new(profile))
    }

    pub fn get(&self) -> Cow<Profile> {
        match self {
            Self::BuiltIn(profile) => Cow::Owned(profile.get()),
            Self::External(profile) => Cow::Borrowed(&profile.profile),
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::BuiltIn(_) => None,
            Self::External(profile) => Some(&profile.path),
        }
    }
}

pub struct Overrides {
    pub profile: Option<String>,
    pub handlers: Handlers,
    pub machine: Option<Machine>,
    pub backtrace: Backtrace,
    pub battery: BatteryConfig,
    pub panic: bool,
}

impl Overrides {
    pub const DEFAULT: Self = Self {
        profile: None,
        handlers: Handlers::DEFAULT,
        machine: None,
        backtrace: Backtrace::DEFAULT,
        battery: BatteryConfig::DEFAULT,
        panic: false,
    };
}

impl Default for Overrides {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Serialize, Deserialize)]
pub struct Handlers {
    pub default: Option<Handler>,
    pub battery_conservation: Option<Handler>,
    pub rapid_charging: Option<Handler>,
}

impl Handlers {
    pub const DEFAULT: Self = Self {
        default: None,
        battery_conservation: None,
        rapid_charging: None,
    };

    pub fn default(&self) -> Handler {
        self.default.unwrap_or(Handler::Switch)
    }

    pub fn battery_conservation(&self) -> Handler {
        self.battery_conservation.unwrap_or_else(|| self.default())
    }

    pub fn rapid_charging(&self) -> Handler {
        self.rapid_charging.unwrap_or_else(|| self.default())
    }
}

impl Default for Handlers {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Machine {
    Always,
    Never,
    Auto,
}

impl Default for Machine {
    fn default() -> Self {
        Machine::Auto
    }
}

impl Machine {
    pub fn get(self) -> bool {
        match self {
            Self::Always => {
                debug!("always machine");
                true
            }
            Self::Never => {
                debug!("never machine");
                false
            }
            Self::Auto => {
                let isnt_a_tty = atty::isnt(atty::Stream::Stdout);

                if isnt_a_tty {
                    debug!("isn't a tty, so machine");
                } else {
                    debug!("is a tty, so not machine");
                }

                isnt_a_tty
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Backtrace {
    pub panics: bool,
    pub errors: bool,
}

impl Default for Backtrace {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Backtrace {
    const DEFAULT: Self = Self {
        panics: false,
        errors: false,
    };
    const RUST_BACKTRACE: &'static str = "RUST_BACKTRACE";
    const RUST_LIB_BACKTRACE: &'static str = "RUST_LIB_BACKTRACE";

    pub fn configure(&self) {
        debug!("backtrace configuration");

        if self.panics {
            debug!("enable backtrace on panic")
        } else {
            debug!("disable backtrace on panic")
        }

        if self.errors {
            debug!("enable backtrace on error")
        } else {
            debug!("disable backtrace on error")
        }

        let configuration = match (self.panics, self.errors) {
            (true, true) => &[(Self::RUST_BACKTRACE, "1")][..],
            (false, true) => &[(Self::RUST_LIB_BACKTRACE, "1")][..],
            (true, false) => &[(Self::RUST_BACKTRACE, "1"), (Self::RUST_LIB_BACKTRACE, "0")][..],
            (false, false) => &[(Self::RUST_BACKTRACE, "0"), (Self::RUST_LIB_BACKTRACE, "0")][..],
        };

        for &(key, value) in configuration {
            debug!("set '{}' to '{}'", key, value);
            env::set_var(key, value)
        }
    }
}

impl fmt::Display for Backtrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{}", self.panics as u8, self.errors as u8)
    }
}

impl FromStr for Backtrace {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (panics, errors) = s.split_once(',').context("expected ',' delimiter")?;
        let panics = panics
            .parse::<u8>()
            .context("expected 'panics' to be a number")?
            != 0;
        let errors = errors
            .parse::<u8>()
            .context("expected 'errors' to be a number")?
            != 0;

        Ok(Self { panics, errors })
    }
}

impl Not for Machine {
    type Output = bool;

    fn not(self) -> Self::Output {
        !self.get()
    }
}

impl FromStr for Machine {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "always" | "true" | "t" => Self::Always,
            "never" | "false" | "f" => Self::Never,
            "auto" | "a" => Self::Auto,
            _ => anyhow::bail!("invalid machine choice '{}'", s),
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct BatteryLevel(u8);

impl BatteryLevel {
    pub const DEFAULT: Self = Self(80);
    const RANGE: RangeInclusive<u8> = 0..=100;

    pub fn new(level: u8) -> Option<Self> {
        if Self::RANGE.contains(&level) {
            Some(Self(level))
        } else {
            None
        }
    }

    pub const fn inner(self) -> u8 { self.0 }
}

impl FromStr for BatteryLevel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s
            .trim_end_matches('%')
            .parse::<u8>()
            .context("number given wasn't valid")?
            .pipe(Self::new)
            .with_context(|| format!("{}% is out of bounds (must be within 0 and 100 inclusive)", s))
    }
}

impl Default for BatteryLevel {
    fn default() -> Self {
        Self(80)
    }
}

impl fmt::Display for BatteryLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}%", self.0)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct CoolDown(pub Duration);

impl CoolDown {
    pub const DEFAULT: Self = Self(Duration::from_secs(60));
}

impl FromStr for CoolDown {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        pub enum F64OrU64 { F64(f64), U64(u64) }

        impl F64OrU64 {
            pub fn into_duration(self) -> Duration {
                match self {
                    Self::F64(value) => Duration::from_secs_f64(value),
                    Self::U64(value) => Duration::from_secs(value),
                }
            }
        }

        let s = s.trim_end_matches('s');

        s.parse::<f64>()
            .map(F64OrU64::F64)
            .or_else(|_| s.parse::<u64>()
                .map(F64OrU64::U64)
            )
            .map(F64OrU64::into_duration)
            .map(Self)
            .context("value wasn't a valid double or number")
    }
}

impl Default for CoolDown {
    fn default() -> Self {
        Self(Duration::from_secs(60))
    }
}

impl fmt::Display for CoolDown {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let duration = self.0.as_secs_f64();

        if duration.fract() == 0.0 {
            (duration as u64).fmt(f)
        } else {
            duration.fmt(f)
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum BatteryMatches {
    First,
    Index(usize),
    Vendor(String),
    Model(String),
    SerialNumber(String),
}

impl BatteryMatches {
    pub fn find(&self, batteries: &mut Batteries) -> anyhow::Result<Option<Battery>> {
        batteries.collect::<Result<Vec<_>, _>>()
            .context("failed to get list of batteries")?
            .into_iter()
            .enumerate()
            .find(|(index, battery)| self.matches(*index, battery))
            .map(|(_, battery)| battery)
            .pipe(Ok)
    }

    pub fn find_infallible(&self, batteries: &mut Batteries) -> (Option<Battery>, Vec<anyhow::Error>) {
        let mut errors = Vec::new();
        let mut battery = None;

        for (index, enumerated_battery) in batteries.enumerate() {
            let enumerated_battery = match enumerated_battery.context("failed to get battery") {
                Ok(battery) => battery,
                Err(error) => {
                    errors.push(error);
                    continue;
                }
            };

            if self.matches(index, &enumerated_battery) {
                battery = Some(enumerated_battery);
                break
            }
        }

        (battery, errors)
    }

    fn matches(&self, index: usize, battery: &Battery) -> bool {
        match self {
            BatteryMatches::First if index == 0 => true,
            BatteryMatches::First => false,
            BatteryMatches::Index(this_index) => *this_index == index,
            BatteryMatches::Vendor(vendor) => battery.vendor().map(|v| v == vendor).unwrap_or(false),
            BatteryMatches::Model(model) => battery.model().map(|m| m == model).unwrap_or(false),
            BatteryMatches::SerialNumber(serial_number) => battery.serial_number().map(|s| s == serial_number).unwrap_or(false),
        }
    }
}

impl FromStr for BatteryMatches {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.split_once('=').context("delimit the variant and value with '='")? {
            ("first" | "f", _) => Ok(BatteryMatches::First),
            ("index" | "i", value) => value.parse()
                .context("value wasn't a valid integer")
                .map(BatteryMatches::Index),
            ("vendor" | "v", value) => Ok(BatteryMatches::Vendor(value.to_string())),
            ("model" | "m", value) => Ok(BatteryMatches::Model(value.to_string())),
            ("serial_number" | "sn" | "s", value) => Ok(BatteryMatches::SerialNumber(value.to_string())),
            (variant, value) => anyhow::bail!("unknown variant '{}' with value '{}' passed", variant, value)
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct BatteryConfig {
    pub matches: Option<BatteryMatches>,
    pub infallible: bool,
    pub threshold: Option<FromStrDeserializer<DisplaySerializer<BatteryLevel>>>,
    pub cooldown: Option<FromStrDeserializer<DisplaySerializer<CoolDown>>>,
}

impl BatteryConfig {
    pub const DEFAULT: Self = Self {
        matches: None,
        infallible: false,
        threshold: None,
        cooldown: None,
    };

    pub fn matches(&self) -> Cow<BatteryMatches> {
        self.matches
            .as_ref()
            .map(Cow::Borrowed)
            .unwrap_or_else(|| Cow::Owned(BatteryMatches::First))
    }

    pub fn threshold(&self) -> BatteryLevel {
        self.threshold
            .map(|threshold| threshold.0.0)
            .unwrap_or(BatteryLevel::DEFAULT)
    }

    pub fn cooldown(&self) -> CoolDown {
        self.cooldown
            .map(|cooldown| cooldown.0.0)
            .unwrap_or(CoolDown::DEFAULT)
    }

    pub fn get(&self) -> anyhow::Result<(Option<Battery>, Vec<anyhow::Error>)> {
        debug!("create battery manager");
        let manager = battery::Manager::new()
            .context("failed to create battery manager")?;

        debug!("create battery iterator");
        let mut batteries = manager.batteries()
            .context("failed to get batteries")?;
        let matches = self.matches();

        let (battery, errors) = if self.infallible {
            matches.find_infallible(&mut batteries)
        } else {
            let battery = matches.find(&mut batteries)
                .context("failed to find battery")?;

            (battery, Vec::new())
        };

        Ok((battery, errors))
    }
}

impl Default for BatteryConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Serialize, Deserialize)]
pub struct TuxVantage {
    pub profile: Option<String>,
    pub machine: Option<Machine>,

    #[serde(default)]
    pub panic: bool,

    #[serde(default)]
    pub handlers: Handlers,

    #[serde(default)]
    pub backtrace: Backtrace,

    #[serde(default)]
    pub battery: BatteryConfig,

    #[serde(skip)]
    pub overrides: Overrides,
}

impl TuxVantage {
    pub const DEFAULT: Self = Self {
        profile: None,
        handlers: Handlers::DEFAULT,
        panic: false,
        machine: None,
        backtrace: Backtrace::DEFAULT,
        battery: BatteryConfig::DEFAULT,
        overrides: Overrides::DEFAULT,
    };

    pub fn get() -> anyhow::Result<Self> {
        project_paths::tuxvantage_toml()
            .pipe(fs::read_to_string)
            .context("failed to read `tuxvantage.toml`")?
            .pipe_deref(toml::from_str)
            .context("failed to deserialize contents of `tuxvantage.toml`")
    }

    pub fn profile(&self) -> Option<&str> {
        self.overrides
            .profile
            .as_deref()
            .or_else(|| self.profile.as_deref())
    }

    pub fn machine(&self) -> Machine {
        self.overrides.machine.unwrap_or_else(|| {
            debug!("no override for machine given, using config");
            self.machine.unwrap_or_else(|| {
                debug!("no machine given in config, using default");
                Machine::Auto
            })
        })
    }

    pub fn battery_config(&self) -> BatteryConfig {
        BatteryConfig {
            matches: self.overrides.battery.matches.clone().or_else(|| self.battery.matches.clone()),
            infallible: self.overrides.battery.infallible || self.battery.infallible,
            threshold: self.overrides.battery.threshold.or(self.battery.threshold),
            cooldown: self.overrides.battery.cooldown.or(self.battery.cooldown),
        }
    }

    pub fn battery(&self) -> anyhow::Result<(Option<Battery>, Vec<anyhow::Error>)> {
        self.battery_config().get()
    }

    pub fn panic(&self) -> bool {
        self.overrides.panic || self.panic
    }

    pub fn backtrace(&self) -> Backtrace {
        let panics = self.overrides.backtrace.panics || self.backtrace.panics;
        let errors = self.overrides.backtrace.errors || self.backtrace.errors;

        Backtrace { panics, errors }
    }

    pub fn handlers(&self) -> Handlers {
        let default = self.overrides.handlers.default.or(self.handlers.default);
        let battery_conservation = self
            .overrides
            .handlers
            .battery_conservation
            .or(self.handlers.battery_conservation);
        let rapid_charging = self
            .overrides
            .handlers
            .rapid_charging
            .or(self.handlers.rapid_charging);

        Handlers {
            default,
            battery_conservation,
            rapid_charging,
        }
    }

    pub fn dump(&self) -> anyhow::Result<()> {
        let tuxvantage_toml = project_paths::tuxvantage_toml();

        let contents = self
            .pipe_ref(toml::to_string)
            .context("failed to serialize the config")?;

        fs::write(tuxvantage_toml, contents).context("failed to write to `tuxvantage.toml`")
    }
}

pub struct Profiles(pub Vec<ExternalProfile>);

impl Profiles {
    pub fn get() -> anyhow::Result<(Self, Vec<anyhow::Error>)> {
        let mut errors = Vec::new();
        let mut profiles = Vec::new();

        for profile in
            project_paths::profiles().context("failed to get handle to profiles directory")?
        {
            match profile {
                Ok(profile) => profiles.push(profile),
                Err(err) => errors.push(err),
            }
        }

        Ok((Self(profiles), errors))
    }

    pub fn find(&self, name: &str) -> Option<Profile> {
        self.with_built_ins()
            .map(|profile| profile.get().deref().clone())
            .find(|profile| profile.name == name)
    }

    pub fn with_built_ins(&self) -> impl Iterator<Item = PossiblyBuiltInProfile> + '_ {
        [BuiltInProfile::Ideapad15IIL05, BuiltInProfile::Ideapad15Amd]
            .into_iter()
            .map(PossiblyBuiltInProfile::BuiltIn)
            .chain(self.0.iter().cloned().map(PossiblyBuiltInProfile::external))
    }
}

static CONFIG: OnceCell<RwLock<Config>> = OnceCell::new();

pub struct Config {
    pub tuxvantage: TuxVantage,
    pub profiles: Profiles,
}

impl Config {
    pub fn ensure_exists() -> anyhow::Result<()> {
        debug!("ensure that the config exists");

        debug!("try create config directory");
        project_paths::config_dir()
            .tap(|path| debug!("config directory is in path '{}'", path.display()))
            .pipe(fs::create_dir_all)
            .context("failed to create the config directory")?;

        debug!("try create profile directory");
        project_paths::profiles_dir()
            .tap(|path| debug!("profile directory is in path '{}'", path.display()))
            .pipe(fs::create_dir_all)
            .context("failed to create the profiles directory")?;

        let tuxvantage_toml = project_paths::tuxvantage_toml();
        debug!("`tuxvantage.toml` exists in '{}'", tuxvantage_toml.display());

        if !tuxvantage_toml.exists() {
            debug!("serialize default `tuxvantage.toml`");
            let contents = TuxVantage::DEFAULT
                .pipe_ref(toml::to_string)
                .expect("failed to serialize the default config");

            debug!("write default `tuxvantage.toml` to path");
            fs::write(tuxvantage_toml, contents)
                .context("failed to write to `tuxvantage.toml`")?;
        }

        EXISTENCE_ENSURED.store(true, Ordering::SeqCst);
        debug!("existence was ensured");

        Ok(())
    }

    pub fn get() -> anyhow::Result<(Self, Vec<anyhow::Error>)> {
        if !EXISTENCE_ENSURED.load(Ordering::SeqCst) {
            Self::ensure_exists()?;
        }
        let (profiles, errors) = Profiles::get().context("failed to get profiles")?;

        Ok((
            Self {
                tuxvantage: TuxVantage::get().context("failed to get `tuxvantage.toml`")?,
                profiles,
            },
            errors,
        ))
    }

    pub fn initialize() -> anyhow::Result<Vec<anyhow::Error>> {
        if CONFIG.get().is_none() {
            let (this, errors) = Self::get()?;
            let _ = CONFIG.set(RwLock::new(this));
            Ok(errors)
        } else {
            Ok(Vec::new())
        }
    }

    fn load() -> &'static RwLock<Self> {
        CONFIG.get().expect("config not initialized")
    }

    pub fn read() -> RwLockReadGuard<'static, Self> {
        Self::load().read()
    }

    pub fn write() -> RwLockWriteGuard<'static, Self> {
        Self::load().write()
    }

    pub fn default_profile(&self) -> Option<anyhow::Result<Profile>> {
        fn inner(this: &Config, profile: &str) -> anyhow::Result<Profile> {
            this.profiles
                .find(profile)
                .with_context(|| format!("the default profile '{}' does not exist", profile))
        }

        Some(inner(self, self.tuxvantage.profile()?))
    }
}

pub fn read() -> RwLockReadGuard<'static, Config> {
    Config::read()
}

pub fn write() -> RwLockWriteGuard<'static, Config> {
    Config::write()
}

pub fn initialize() -> anyhow::Result<Vec<anyhow::Error>> {
    Config::initialize()
}

pub fn machine() -> Machine {
    read().tuxvantage.machine()
}
