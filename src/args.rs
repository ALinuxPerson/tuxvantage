use std::str::FromStr;

use crate::config::{Backtrace, BatteryLevel, BatteryMatches, CoolDown, Machine};
use clap::Parser;
use ideapad::{Handler, SystemPerformanceMode};

#[derive(Debug)]
pub struct FromStrHandler(pub Handler);

impl FromStr for FromStrHandler {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ignore" | "i" => Ok(Self(Handler::Ignore)),
            "error" | "e" => Ok(Self(Handler::Error)),
            "switch" | "s" => Ok(Self(Handler::Switch)),
            _ => anyhow::bail!("invalid handler '{}'", s),
        }
    }
}

#[derive(Debug)]
pub struct FromStrSystemPerformanceMode(pub SystemPerformanceMode);

impl FromStr for FromStrSystemPerformanceMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "intelligent-cooling" | "ic" | "i" => {
                Ok(Self(SystemPerformanceMode::IntelligentCooling))
            }
            "extreme-performance" | "ep" | "e" => {
                Ok(Self(SystemPerformanceMode::ExtremePerformance))
            }
            "battery-saving" | "bs" | "b" => Ok(Self(SystemPerformanceMode::BatterySaving)),
            _ => anyhow::bail!("invalid system performance mode '{}'", s),
        }
    }
}

/// A utility which brings some Windows exclusive functionality of the Lenovo Vantage software
/// to Linux systems. Or... erhm... gives Linux the TuxVantage (not trademarked)
#[derive(Debug, Parser)]
#[clap(about, version, author)]
pub struct TuxVantage {
    /// The name of the profile to use. Overrides the config file.
    #[clap(short, long)]
    pub profile: Option<String>,

    /// Enable machine readable output for robots. Overrides the config file.
    #[clap(short, long)]
    pub machine: Option<Machine>,

    /// Panic on error. Should be used for debugging purposes only. Overrides the config file.
    #[clap(short = 'P', long)]
    pub panic: bool,

    /// Set the backtrace configuration. In the format "[panics (0 or 1)],[errors (0 or 1)]".
    /// Overrides the config file.
    #[clap(short, long, default_value_t)]
    pub backtrace: Backtrace,

    /// The handler to use. If not passed, it will use the config file, and if it isn't passed
    /// there either, it will use `switch`. Overrides the config file.
    #[clap(short, long)]
    pub handler: Option<FromStrHandler>,

    /// Enable verbose output.
    #[clap(short, long)]
    pub verbose: bool,

    /// Skip consistency checks. Should be used for debugging purposes only.
    #[clap(long)]
    pub skip_consistency_checks: bool,

    #[clap(subcommand)]
    pub action: TuxVantageAction,
}

#[derive(Debug, Parser)]
pub enum TuxVantageAction {
    /// Manage battery conservation mode.
    #[clap(subcommand)]
    BatteryConservation(TuxVantageBatteryConservation),

    /// Manage the system performance mode.
    #[clap(subcommand)]
    SystemPerformance(TuxVantageSystemPerformance),

    /// Manage rapid charging.
    #[clap(subcommand)]
    RapidCharge(TuxVantageRapidCharge),

    /// Manage the profiles.
    #[clap(subcommand)]
    Profiles(TuxVantageProfiles),
}

#[derive(Debug, Parser)]
#[clap(visible_aliases = &["bc", "b"])]
pub enum TuxVantageBatteryConservation {
    /// Check if battery conservation mode is enabled.
    #[clap(visible_aliases = &["ie", "g"])]
    Enabled,

    /// Check if battery conservation mode is disabled.
    #[clap(visible_alias = "id")]
    Disabled,

    /// Enable battery conservation mode.
    #[clap(visible_alias = "e")]
    Enable {
        /// What to do if rapid charging is enabled. If not specified, the default would be
        /// chosen from the config. If there is no default specified there, the default would
        /// be `switch`.
        handler: Option<FromStrHandler>,
    },

    /// Disable battery conservation mode.
    #[clap(visible_alias = "d")]
    Disable,

    /// Regulate the battery using battery conservation mode.
    #[clap(visible_alias = "r")]
    Regulate {
        /// The target battery level in which battery conservation mode will be enabled.
        #[clap(short, long, default_value_t)]
        threshold: BatteryLevel,

        /// How long to wait to check the battery level again.
        #[clap(short, long, default_value_t)]
        cooldown: CoolDown,

        /// Do not error if an error occurred while enumerating a battery. Instead, display a
        /// warning.
        #[clap(short, long)]
        infallible: bool,

        /// How to find the desired battery.
        #[clap(short, long)]
        matches: Option<BatteryMatches>,

        /// Install the battery regulation service. Assumes you're using SystemD.
        #[clap(short = 'I', long)]
        install: bool,
    },
}

#[derive(Debug, Parser)]
#[clap(visible_aliases = &["sp", "s"])]
pub enum TuxVantageSystemPerformance {
    /// Get the current system performance mode.
    #[clap(visible_alias = "g")]
    Get,

    /// Set the system performance mode.
    #[clap(visible_alias = "s")]
    Set {
        /// The system performance mode to set.
        mode: FromStrSystemPerformanceMode,
    },
}

#[derive(Debug, Parser)]
#[clap(visible_aliases = &["rc", "r"])]
pub enum TuxVantageRapidCharge {
    /// Check if rapid charging is enabled.
    #[clap(visible_aliases = &["ie", "g"])]
    Enabled,

    /// Check if rapid charging is disabled.
    #[clap(visible_alias = "id")]
    Disabled,

    /// Enable rapid charging.
    #[clap(visible_alias = "e")]
    Enable {
        /// What to do if battery conservation is enabled. If not specified, the default would
        /// be chosen from the config. If there is no default specified there, the default would
        /// be `switch`.
        handler: Option<FromStrHandler>,
    },

    /// Disable rapid charging.
    #[clap(visible_alias = "d")]
    Disable,
}

#[derive(Debug, Parser)]
#[clap(visible_alias = "p")]
pub enum TuxVantageProfiles {
    /// Get a profile.
    #[clap(visible_alias = "g")]
    Get {
        /// The profile to get. If not given, all profiles will be listed.
        name: Option<String>,
    },

    /// Get the default profile from the config file. If there is no default specified there,
    /// the auto-detected profile will be returned.
    #[clap(visible_alias = "gd")]
    GetDefault,

    /// Set a profile.
    #[clap(visible_alias = "s")]
    Set {
        /// The name of the profile to set.
        name: String,

        /// The contents of the profile in JSON. If this is not given, standard input will be
        /// used.
        contents: Option<String>,

        /// Create a new profile if the given name doesn't exist.
        #[clap(short, long)]
        create_new: bool,
    },

    /// Set the default profile.
    #[clap(visible_alias = "sd")]
    SetDefault {
        /// The name of the profile to set as the default.
        name: String,
    },

    /// Delete a profile.
    #[clap(visible_aliases = &["r", "rm"])]
    Remove {
        /// The name of the profile to remove. If the name of the profile given was the default,
        /// this will be changed to the auto-detected profile.
        name: String,
    },

    /// Get the JSON contents of a profile.
    #[clap(visible_alias = "j")]
    Json {
        /// The name of the profile to get the JSON contents of.
        name: String,

        /// If reading the JSON contents of a profile fails, generate it instead.
        #[clap(short, long)]
        generate_on_error: bool,

        /// Prettify the JSON contents of a profile. Only works if the profile JSON needs to be
        /// generated.
        #[clap(short, long)]
        pretty: bool,
    },
}

pub fn parse() -> TuxVantage {
    TuxVantage::parse()
}
