mod private {
    pub trait Sealed {}
}

use crate::app::IntoOptionMachineOutput;
use ::log::LevelFilter;
use anyhow::{anyhow, Context};
use ideapad::Handler;
use owo_colors::OwoColorize;
use parking_lot::RwLockWriteGuard;
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use std::path::Path;
use std::process::Command;
use std::{env, fs, thread};


use crate::args::FromStrHandler;
use crate::config::{BatteryConfig, BatteryLevel, BatteryMatches, CoolDown};
use crate::ext::AnyhowResultExt;
use crate::log::Level;
use crate::utils::{DisplaySerializer, FromStrDeserializer};
use crate::{anyhow_with_tip, config, context, log, utils, verbose};

#[derive(Serialize)]
#[serde(untagged)]
pub enum MachineOutput {
    Enabled { enabled: bool },
    Disabled { disabled: bool },
}

impl IntoOptionMachineOutput<MachineOutput> for MachineOutput {
    fn into_option_machine_output(self) -> Option<MachineOutput> {
        Some(self)
    }
}

pub fn enabled() -> anyhow_with_tip::Result<MachineOutput> {
    debug!("get battery conservation enabled value");
    let enabled = ideapad::battery_conservation::enabled(context::get())
        .context("failed to get battery conservation mode value")
        .maybe_acpi_call_tip()?;
    let what = if enabled {
        "enabled".bold().green().to_string()
    } else {
        "disabled".bold().red().to_string()
    };

    if !config::machine() {
        info!("battery conservation is {}", what);
    }

    Ok(MachineOutput::Enabled { enabled })
}

pub fn disabled() -> anyhow_with_tip::Result<MachineOutput> {
    debug!("get battery conservation disabled value");
    let disabled = ideapad::battery_conservation::disabled(context::get())
        .context("failed to get battery conservation mode value")
        .maybe_acpi_call_tip()?;
    let what = if disabled {
        "disabled".bold().green().to_string()
    } else {
        "enabled".bold().red().to_string()
    };

    if !config::machine() {
        info!("battery conservation is {}", what);
    }

    Ok(MachineOutput::Disabled { disabled })
}

pub fn enable(handler: Option<FromStrHandler>) -> anyhow_with_tip::Result<()> {
    let mut config = config::write();

    debug!("setup argument override for battery conservation handler from config");
    config.tuxvantage.overrides.handlers.battery_conservation = handler.map(|handler| handler.0);

    let config = RwLockWriteGuard::downgrade(config);
    let handler = config.tuxvantage.handlers().battery_conservation();
    let machine = config.tuxvantage.machine();

    if !machine {
        info!(
            "trying to enable battery conservation with handler {}",
            super::format_handler(handler)
        );

        if let Handler::Ignore = handler {
            warn!("use this handler with care; if rapid charge is already enabled this will strain the battery")
        }
    }

    debug!("enable battery conservation with handler {:?}", handler);
    context::get()
        .controllers()
        .battery_conservation()
        .enable()
        .handler(handler)
        .now()
        .context("failed to enable battery conservation")
        .maybe_acpi_call_tip()?;

    if !machine {
        info!("enabled battery conservation");
    }

    Ok(())
}

pub fn disable() -> anyhow_with_tip::Result<()> {
    debug!("disable battery conservation");
    ideapad::battery_conservation::disable(context::get())
        .context("failed to disable battery conservation")
        .maybe_acpi_call_tip()?;

    if !config::machine() {
        info!("disabled battery conservation");
    }

    Ok(())
}

pub fn regulate(
    threshold: BatteryLevel,
    cooldown: CoolDown,
    infallible: bool,
    matches: Option<BatteryMatches>,
    install: bool,
) -> anyhow_with_tip::Result<()> {
    let mut config = config::write();

    if install {
        if !utils::is_systemd()? {
            return Err(anyhow::anyhow!(
                "you can only install this service on systems which use the systemd init system"
            )
            .into());
        }

        // todo: allow changing of the service name
        let path = Path::new("/etc/systemd/system/bcm.service");
        info!(
            "installing battery conservation regulator service to {}",
            path.display().bold()
        );

        // todo: maybe we could put a consistency check here in some hidden configuration file?
        let tuxvantage_exe =
            env::current_exe().context("failed to get current path to executable")?;

        debug!("path to tuxvantage exe is: {}", tuxvantage_exe.display());

        let tuxvantage_exe_str = tuxvantage_exe.to_str().with_context(|| {
            format!(
                "path to tuxvantage ({}) contains invalid utf-8",
                tuxvantage_exe.display().bold()
            )
        })?;

        let contents = format!(
            include_str!("../../assets/bcm.service"),
            tuxvantage_exe = tuxvantage_exe_str,
        );

        debug!("contents to write are:\n {}", contents);

        fs::write(path, contents).context("failed to write content into file")?;

        debug!("setting regulator service installed bit to be true");
        config
            .consistency
            .mutate_then_dump(move |consistency| {
                consistency.regulator_service_installed = true;
                consistency.last_exe = Some(tuxvantage_exe);
            })
            .context("failed to dump consistency configuration")?;

        info!("reloading the systemd daemon");
        let daemon_reload_successful = Command::new("systemctl")
            .arg("daemon-reload")
            .spawn()
            .context("failed to reload the systemd daemon (command was systemctl daemon-reload)")?
            .wait()
            .context("failed to wait on reloading the systemd daemon (command was systemctl daemon-reload)")?
            .success();

        if !daemon_reload_successful {
            return Err(anyhow::anyhow!("reloading the systemd daemon wasn't successful").into());
        }

        return Ok(());
    }

    config.tuxvantage.overrides.battery = BatteryConfig {
        threshold: Some(FromStrDeserializer(DisplaySerializer(threshold))),
        cooldown: Some(FromStrDeserializer(DisplaySerializer(cooldown))),
        infallible,
        matches,
    };
    let battery_config = config.tuxvantage.battery_config();
    let (battery, errors) = battery_config.get().context("failed to get battery")?;

    if !errors.is_empty() {
        warn!("errors occurred while retrieving battery information, see below");

        for error in errors {
            warn!("{}", error);
        }
    }

    let mut battery = match battery {
        Some(battery) => battery,
        None => {
            info!("failed to get battery information, here are the list of batteries that you could use");
            let r#try = || -> anyhow::Result<()> {
                let manager =
                    battery::Manager::new().context("failed to create battery manager")?;
                let batteries = manager
                    .batteries()
                    .context("failed to get list of batteries")?;
                let mut first = true;

                {
                    let _guard = log::no_prologue::guard_for(Level::Info);
                    for (index, battery) in batteries.enumerate() {
                        let battery = match battery {
                            Ok(battery) => battery,
                            Err(error) => {
                                warn!("skipping battery due to an error: {}", error);
                                continue;
                            }
                        };

                        if first {
                            info!(
                                "{} {}",
                                format_args!("#{}", index).bold(),
                                "(first)".italic()
                            );
                            first = false
                        } else {
                            info!("{}", format_args!("#{}", index).bold())
                        }

                        info!(
                            "{}{} {}",
                            super::tab(2),
                            "Vendor".bold(),
                            battery.vendor().unwrap_or("N/A")
                        );
                        info!(
                            "{}{} {}",
                            super::tab(2),
                            "Model".bold(),
                            battery.model().unwrap_or("N/A")
                        );
                        info!(
                            "{}{} {}",
                            super::tab(2),
                            "Serial Number".bold(),
                            battery.serial_number().unwrap_or("N/A")
                        );
                    }
                }

                Ok(())
            };

            if let Err(error) = r#try() {
                let error = error.context("failed to display list of batteries");
                warn!("{:#}", error)
            }

            return Err(anyhow!("failed to get battery information").into());
        }
    };

    let level_filter = if verbose::get() {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    env_logger::Builder::new().filter_level(level_filter).init();

    let cooldown = battery_config.cooldown().0;
    let threshold = battery_config.threshold().inner();
    let handler = config.tuxvantage.handlers().battery_conservation();
    let mut battery_conservation = context::get().controllers().battery_conservation();

    ::log::info!(
        "the cooldown is {} second(s)",
        cooldown.as_secs_f64().bold()
    );
    ::log::info!(
        "the threshold for the battery is {}",
        format_args!("{}%", threshold.bold())
    );

    let (signal_sender, signal_receiver) = crossbeam::channel::bounded(1);
    let mut signals = Signals::new([SIGTERM, SIGINT])
        .context("failed to register handler for application exits")?;

    thread::spawn(move || {
        for _ in signals.forever() {
            signal_sender.send(()).unwrap();
        }
    });

    loop {
        let battery_level = (battery.state_of_charge().value * 100.0).round() as u8;
        ::log::info!(
            "current battery level is {}",
            format_args!("{}%", battery_level.bold())
        );

        let battery_level_ge_threshold = battery_level >= threshold;
        ::log::debug!(
            "battery level >= threshold = {}",
            battery_level_ge_threshold
        );

        if battery_level >= threshold {
            ::log::info!("battery level is greater than or equal to the provided threshold, enabling battery conservation mode");
            battery_conservation
                .enable()
                .handler(handler)
                .now()
                .context("failed to enable battery conservation")
                .maybe_acpi_call_tip()?
        } else {
            ::log::info!("battery level is less than the provided threshold, disabling battery conservation mode");
            battery_conservation
                .disable()
                .context("failed to disable battery conservation")
                .maybe_acpi_call_tip()?
        }

        ::log::info!("refreshing battery");
        if let Err(error) = battery.refresh() {
            ::log::warn!("failed to refresh battery: {}", error)
        }

        ::log::debug!("sleeping for {} second(s)", cooldown.as_secs_f64().bold());
        let sleep_receiver = utils::sleep(cooldown);

        crossbeam::select! {
            recv(sleep_receiver) -> _ => continue,
            recv(signal_receiver) -> _ => {
                ::log::info!("received signal to terminate the current program, exiting cleanly");
                ::log::info!("enabling battery conservation mode");

                battery_conservation
                    .enable()
                    .handler(handler)
                    .now()
                    .context("failed to enable battery conservation")
                    .maybe_acpi_call_tip()?;
                break Ok(())
            }
        }
    }
}
