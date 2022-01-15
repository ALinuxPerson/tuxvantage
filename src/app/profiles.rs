use crate::app::IntoOptionMachineOutput;
use crate::config::PossiblyBuiltInProfile;
use crate::project_paths::profiles::ExternalProfile;
use crate::{anyhow_with_tip, config, log, project_paths, TippingAnyhowResultExt};
use anyhow::Context;
use ideapad::{Profile, profile::Bit};
use owo_colors::OwoColorize;
use std::io::Read;
use std::ops::Deref;
use std::{fmt, fs, io};
use ideapad::profile::BitInner;

fn format_bits(name: impl fmt::Display, bit: Bit, indent: usize) {
    if let BitInner::Same(_) = bit.inner() {
        info!(
            "{}{} {}",
            super::tab(indent),
            name.bold(),
            "(bits are the same)".italic()
        );
        info!(
            "{}{} {} {}",
            super::tab(indent + 1),
            "FCMO/SPMO Bit".bold(),
            bit.fcmo(),
            format_args!("({:#04x})", bit.spmo()).italic()
        );
    } else {
        info!("{}{}", super::tab(indent), name.bold());
        info!(
            "{}{} {} {}",
            super::tab(indent + 1),
            "FCMO Bit".bold(),
            bit.fcmo(),
            format_args!("({:#04x})", bit.fcmo()).italic()
        );
        info!(
            "{}{} {} {}",
            super::tab(indent + 1),
            "SPMO Bit".bold(),
            bit.spmo(),
            format_args!("({:#04x})", bit.spmo()).italic()
        );
    }
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum MachineOutput {
    Get { profiles: Vec<Profile> },
    Json { json: String },
}

impl IntoOptionMachineOutput<MachineOutput> for MachineOutput {
    fn into_option_machine_output(self) -> Option<MachineOutput> {
        Some(self)
    }
}

pub fn get(name: Option<String>) -> anyhow::Result<MachineOutput> {
    let config = config::read();
    let profiles = &config.profiles;

    debug!("check if we should show all profiles or `name`");
    let (profiles, is_singular) = match &name {
        Some(name) => {
            debug!("name is set, so we should show only show that profile");
            debug!("check if a profile with that name '{}' exists", name);
            let profile = profiles
                .with_built_ins()
                .find(|profile| &profile.get().name == name)
                .with_context(|| format!("profile '{}' not found", name))?;

            (vec![profile], true)
        }
        None => {
            debug!("name is not set, show all profiles");
            (profiles.with_built_ins().collect(), false)
        }
    };
    debug!("is singular = {}", is_singular);

    let machine = config.tuxvantage.machine();

    if !machine {
        if is_singular {
            let name = name.expect("name should always exist when singular");
            info!("profile definition for '{}':", name)
        } else {
            info!("list of profiles and their contents:")
        }

        for possibly_built_in_profile in &profiles {
            debug!("next profile");
            let _guard = log::no_prologue::guard_for(log::Level::Info);
            if !is_singular {
                info!("")
            }
            let mut epilogue = match possibly_built_in_profile {
                PossiblyBuiltInProfile::BuiltIn(_) => {
                    debug!("profile is built-in");
                    "(built-in)"
                }
                PossiblyBuiltInProfile::External { .. } => {
                    debug!("profile is external");
                    "(external)"
                }
            }
            .italic()
            .to_string();

            debug!("epilogue is '{}'", epilogue);

            let profile = possibly_built_in_profile.get();

            if let Some(default) = config.tuxvantage.profile() {
                debug!("get default profile name");
                if default == profile.name {
                    debug!("profile is default, push this notice to epilogue");
                    epilogue.push_str(" (default)".italic().to_string().as_str());
                }
            }

            debug!("show header");
            info!("{}{} {}", super::tab(1), profile.name.bold(), epilogue);

            if let PossiblyBuiltInProfile::External(ref profile) = possibly_built_in_profile {
                debug!("show path of external profile");
                info!(
                    "{}{} {}",
                    super::tab(2),
                    "Path".bold(),
                    profile.path.display()
                );
            }

            debug!("show product names header");
            info!("{}{}", super::tab(2), "Expected Product Names".bold());

            debug!("show product names");
            for product_name in profile.expected_product_names.iter() {
                info!("{}{}", super::tab(3), product_name);
            }

            debug!("show system performance mode header");
            info!("{}{}", super::tab(2), "System Performance Mode".bold());
            debug!("show set command");
            info!(
                "{}{} {}",
                super::tab(3),
                "Set Command".bold(),
                profile.system_performance.commands.set,
            );
            debug!("show fcmo bit command");
            info!(
                "{}{} {}",
                super::tab(3),
                "Get FCMO Bit Command".bold(),
                profile.system_performance.commands.get_fcmo_bit,
            );
            debug!("show spmo bit command");
            info!(
                "{}{} {}",
                super::tab(3),
                "Get SPMO Bit Command".bold(),
                profile.system_performance.commands.get_spmo_bit,
            );
            info!("{}{}", super::tab(3), "Commands To Get Bits".bold());

            let bits = profile.system_performance.bits;
            format_bits("Intelligent Cooling", bits.intelligent_cooling, 4);
            format_bits("Extreme Performance", bits.extreme_performance, 4);
            format_bits("Battery Saving", bits.battery_saving, 4);

            // let parameters = profile.parameters;
            info!("{}{}", super::tab(3), "Parameters".bold());
            info!(
                "{}{} {} {}",
                super::tab(4),
                "Set To Intelligent Cooling".bold(),
                profile.system_performance.parameters.intelligent_cooling,
                format_args!("({:#010x})", profile.system_performance.parameters.intelligent_cooling).italic()
            );
            info!(
                "{}{} {} {}",
                super::tab(4),
                "Set To Extreme Performance".bold(),
                profile.system_performance.parameters.extreme_performance,
                format_args!("({:#010x})", profile.system_performance.parameters.extreme_performance).italic()
            );
            info!(
                "{}{} {} {}",
                super::tab(4),
                "Set To Battery Saving".bold(),
                profile.system_performance.parameters.battery_saving,
                format_args!("({:#010x})", profile.system_performance.parameters.battery_saving).italic()
            );
            info!("{}{}", super::tab(2), "Battery".bold());
            info!(
                "{}{} {}",
                super::tab(3),
                "Set Command".bold(),
                profile.battery.set_command,
            );
            info!("{}{}", super::tab(3), "Battery Conservation".bold());
            info!(
                "{}{} {}",
                super::tab(4),
                "Get Command".bold(),
                profile.battery.conservation.get_command,
            );
            info!("{}{}", super::tab(4), "Parameters".bold());
            info!(
                "{}{} {} {}",
                super::tab(5),
                "Enable".bold(),
                profile.battery.conservation.parameters.enable,
                format_args!("({:#02x})", profile.battery.conservation.parameters.enable).italic()
            );
            info!(
                "{}{} {} {}",
                super::tab(5),
                "Disable".bold(),
                profile.battery.conservation.parameters.disable,
                format_args!("({:#02x})", profile.battery.conservation.parameters.disable).italic()
            );
            info!("{}{}", super::tab(3), "Rapid Charging".bold());
            info!(
                "{}{} {}",
                super::tab(4),
                "Get Command".bold(),
                profile.battery.rapid_charge.get_command,
            );
            info!("{}{}", super::tab(4), "Parameters".bold());
            info!(
                "{}{} {} {}",
                super::tab(5),
                "Enable".bold(),
                profile.battery.rapid_charge.parameters.enable,
                format_args!("({:#02x})", profile.battery.rapid_charge.parameters.enable).italic()
            );
            info!(
                "{}{} {} {}",
                super::tab(5),
                "Disable".bold(),
                profile.battery.rapid_charge.parameters.disable,
                format_args!("({:#02x})", profile.battery.rapid_charge.parameters.disable).italic()
            );
        }
    }

    Ok(MachineOutput::Get {
        profiles: profiles
            .into_iter()
            .map(|profile| profile.get().deref().clone())
            .collect(),
    })
}

pub fn get_default() -> anyhow::Result<MachineOutput> {
    match config::read().default_profile() {
        Some(default_profile) => {
            debug!("default profile in config");
            let default_profile = default_profile.context("failed to get default profile")?;
            get(Some(default_profile.name.to_string()))
        }
        None => {
            debug!("no default profile found in config, bailing out");
            anyhow::bail!("there is no default profile declared in `tuxvantage.toml`")
        }
    }
}

pub fn set(name: String, contents: Option<String>, create_new: bool) -> anyhow::Result<()> {
    enum Source {
        Arguments,
        Stdin,
    }

    debug!("find profile '{}' which may or may not exist", name);
    let profile = config::read()
        .profiles
        .with_built_ins()
        .find(|profile| profile.get().name == name);

    let profile_path = if let Some(profile) = profile {
        debug!("profile '{}' found, getting the path", name);
        profile
            .path()
            .context("you cannot set the contents of a profile that is built-in")?
            .to_path_buf()
    } else if create_new {
        debug!("profile '{}' not found, generating the path", name);
        let file_name = format!("{}.json", name);
        project_paths::profiles_dir().join(file_name)
    } else {
        anyhow::bail!("profile '{}' not found", name)
    };

    debug!("get the contents and source of the contents");
    let (contents, source) = match contents {
        Some(contents) => {
            debug!("contents exist, assume they're a path to something");
            let contents = fs::read_to_string(contents)
                .context("failed to read new profile contents from file")?;
            debug!("...therefore, the source comes from the arguments");
            (contents, Source::Arguments)
        }
        None => {
            debug!("content doesn't exist, assume they're on stdin");

            let mut contents = String::new();
            io::stdin()
                .read_to_string(&mut contents)
                .context("failed to read from stdin")?;

            debug!("...therefore, the source comes from the stdin");
            (contents, Source::Stdin)
        }
    };

    // make sure that `contents` is valid json
    debug!("make sure that `contents` is valid json");
    serde_json::from_str::<Profile>(&contents).context("contents weren't valid json")?;

    debug!("write the contents to the profile");
    fs::write(&profile_path, contents).context("failed to write to profile file")?;

    if !config::machine() {
        match source {
            Source::Arguments => info!("set profile '{}' to contents from arguments", name),
            Source::Stdin => info!("set profile '{}' to contents from stdin", name),
        }
    }

    Ok(())
}

pub fn set_default(name: String) -> anyhow::Result<()> {
    let mut config = config::write();

    // scoped so that `profiles` will get dropped otherwise borrow checker will get mad at
    // us
    {
        debug!("check if name '{}' exists as a profile", name);
        let mut profiles = config.profiles.with_built_ins();
        let name_exists_as_a_profile = profiles.any(|profile| profile.get().name == name);
        anyhow::ensure!(name_exists_as_a_profile, "profile '{}' not found", name);
    }

    debug!("set the default profile to '{}' in memory", name);
    config.tuxvantage.profile = Some(name.clone());

    debug!("write the new default profile to the config");
    config
        .tuxvantage
        .dump()
        .context("failed to write to `tuxvantage.toml`")?;

    if !config.tuxvantage.machine() {
        info!("set the default profile to '{}'", name);
    }

    Ok(())
}

pub fn remove(name: String) -> anyhow_with_tip::Result<()> {
    let path = config::read()
        .profiles
        .0
        .iter()
        .find(|profile| profile.profile.name == name)
        .with_context(|| format!("profile '{}' not found", name))
        .tip("note that you can't remove built-in profiles")?
        .path
        .clone();

    fs::remove_file(path).context("failed to remove profile file")?;

    if !config::machine() {
        info!("removed the profile '{}'", name);
    }

    Ok(())
}

pub fn json(name: String, generate_on_error: bool, pretty: bool) -> anyhow::Result<MachineOutput> {
    let config = config::read();
    let profile = config
        .profiles
        .with_built_ins()
        .find(|profile| profile.get().name == name)
        .with_context(|| format!("profile '{}' not found", name))?;
    let machine = config.tuxvantage.machine();

    match profile {
        PossiblyBuiltInProfile::BuiltIn(profile) => {
            let profile = profile.get();

            if !machine {
                warn!("the profile '{}' is a built-in profile. note that these types of profiles don't actually exist in the filesystem\n\
                   of your computer, but is actually embedded in the binary of the program. as such, their json files don't actually exist, and\n\
                   are generated by the program itself. you may only use this as a reference.", profile.name);
            }

            let json = if pretty {
                serde_json::to_string_pretty(&profile)
                    .expect("failed to generate pretty json from profile")
            } else {
                serde_json::to_string(&profile).expect("failed to generate json from profile")
            };

            if !machine {
                println!("{}", json);
            }

            Ok(MachineOutput::Json { json })
        }
        PossiblyBuiltInProfile::External(profile) => {
            let ExternalProfile { profile, path } = profile.deref();
            let clause = || -> anyhow::Result<MachineOutput> {
                let json =
                    fs::read_to_string(path).context("failed to read contents of profile json")?;
                Ok(MachineOutput::Json { json })
            };

            match clause() {
                Ok(output) => Ok(output),
                Err(error) => {
                    if generate_on_error {
                        if !machine {
                            warn!("{:#}", error);
                            warn!("generating from profile on memory");
                        }
                        let json = if pretty {
                            serde_json::to_string_pretty(&profile)
                                .expect("failed to generate pretty json from profile")
                        } else {
                            serde_json::to_string(&profile)
                                .expect("failed to generate json from profile")
                        };

                        if !machine {
                            println!("{}", json);
                        }

                        Ok(MachineOutput::Json { json })
                    } else {
                        Err(error)
                    }
                }
            }
        }
    }
}
