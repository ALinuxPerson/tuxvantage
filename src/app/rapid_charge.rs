use crate::app::IntoOptionMachineOutput;
use crate::args::FromStrHandler;
use crate::ext::AnyhowResultExt;
use crate::{anyhow_with_tip, config, context};
use anyhow::Context;
use ideapad::Handler;
use owo_colors::OwoColorize;
use parking_lot::RwLockWriteGuard;

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
    let enabled = ideapad::rapid_charge::enabled(context::get())
        .context("failed to get rapid charge value")
        .maybe_acpi_call_tip()?;
    let what = if enabled {
        "enabled".bold().green().to_string()
    } else {
        "disabled".bold().red().to_string()
    };

    if !config::machine() {
        info!("rapid charge is {}", what)
    }

    Ok(MachineOutput::Enabled { enabled })
}

pub fn disabled() -> anyhow_with_tip::Result<MachineOutput> {
    let disabled = ideapad::rapid_charge::disabled(context::get())
        .context("failed to get rapid charge value")
        .maybe_acpi_call_tip()?;
    let what = if disabled {
        "disabled".bold().green().to_string()
    } else {
        "enabled".bold().red().to_string()
    };

    if !config::machine() {
        info!("rapid charge is {}", what);
    }

    Ok(MachineOutput::Disabled { disabled })
}

pub fn enable(handler: Option<FromStrHandler>) -> anyhow_with_tip::Result<()> {
    let mut config = config::write();
    config.tuxvantage.overrides.handlers.rapid_charging = handler.map(|handler| handler.0);
    let config = RwLockWriteGuard::downgrade(config);
    let handler = config.tuxvantage.handlers().rapid_charging();
    let machine = config.tuxvantage.machine();

    if !machine {
        info!(
            "trying to enable rapid charging with handler {}",
            super::format_handler(handler)
        );

        if let Handler::Ignore = handler {
            warn!("use this handler with care; if rapid charge is already enabled this will strain the battery")
        }
    }

    context::get()
        .controllers()
        .rapid_charge()
        .enable()
        .handler(handler)
        .now()
        .context("failed to enable rapid charging")
        .maybe_acpi_call_tip()?;

    if !machine {
        info!("enabled rapid charging");
    }

    Ok(())
}

pub fn disable() -> anyhow_with_tip::Result<()> {
    ideapad::rapid_charge::disable(context::get())
        .context("failed to disable rapid charge")
        .maybe_acpi_call_tip()?;

    if !config::machine() {
        info!("disabled rapid charge")
    }

    Ok(())
}
