use crate::app::IntoOptionMachineOutput;
use crate::args::FromStrSystemPerformanceMode;
use crate::ext::AnyhowResultExt;
use crate::{anyhow_with_tip, config, context};
use anyhow::Context;
use ideapad::SystemPerformanceMode;

#[derive(Serialize)]
#[serde(untagged)]
pub enum MachineOutput {
    Get {
        system_performance_mode: SystemPerformanceMode,
    },
}

impl IntoOptionMachineOutput<MachineOutput> for MachineOutput {
    fn into_option_machine_output(self) -> Option<MachineOutput> {
        Some(self)
    }
}

pub fn get() -> anyhow_with_tip::Result<MachineOutput> {
    let system_performance_mode = ideapad::system_performance::get(context::get())
        .context("failed to get system performance mode")
        .maybe_acpi_call_tip()?;

    if !config::machine() {
        info!(
            "the system performance mode is {}",
            super::format_system_performance_mode(system_performance_mode)
        )
    }

    Ok(MachineOutput::Get {
        system_performance_mode,
    })
}

pub fn set(mode: FromStrSystemPerformanceMode) -> anyhow_with_tip::Result<()> {
    let mode = mode.0;

    ideapad::system_performance::set(context::get(), mode)
        .with_context(|| {
            format!(
                "failed to set the system performance mode to '{}'",
                super::format_system_performance_mode_plain(mode)
            )
        })
        .maybe_acpi_call_tip()?;

    if !config::machine() {
        info!(
            "the system performance mode has been set to {}",
            super::format_system_performance_mode(mode)
        );
    }

    Ok(())
}
