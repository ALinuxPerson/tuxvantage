pub mod battery_conservation;
pub mod profiles;
pub mod rapid_charge;
pub mod system_performance;

use ideapad::{Handler, SystemPerformanceMode};
use owo_colors::OwoColorize;

fn format_handler(handler: Handler) -> String {
    match handler {
        Handler::Switch => "switch",
        Handler::Ignore => "ignore",
        Handler::Error => "error",
    }
    .bold()
    .to_string()
}

fn format_system_performance_mode(mode: SystemPerformanceMode) -> String {
    format_system_performance_mode_plain(mode)
        .bold()
        .to_string()
}

fn format_system_performance_mode_plain(mode: SystemPerformanceMode) -> &'static str {
    match mode {
        SystemPerformanceMode::ExtremePerformance => "extreme performance",
        SystemPerformanceMode::IntelligentCooling => "intelligent cooling",
        SystemPerformanceMode::BatterySaving => "battery saving",
    }
}

fn tab(count: usize) -> String {
    const TAB: &str = "    ";

    TAB.repeat(count - 1)
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum MachineOutput {
    BatteryConservation(battery_conservation::MachineOutput),
    Profiles(profiles::MachineOutput),
    RapidCharge(rapid_charge::MachineOutput),
    SystemPerformance(system_performance::MachineOutput),
}

pub trait IntoOptionMachineOutput<MO> {
    fn into_option_machine_output(self) -> Option<MO>;
}

impl<MO> IntoOptionMachineOutput<MO> for () {
    fn into_option_machine_output(self) -> Option<MO> {
        None
    }
}

impl MachineOutput {
    pub fn battery_conservation<T>(value: T) -> Option<Self>
    where
        T: IntoOptionMachineOutput<battery_conservation::MachineOutput>,
    {
        value
            .into_option_machine_output()
            .map(Self::BatteryConservation)
    }

    pub fn profiles<T>(value: T) -> Option<Self>
    where
        T: IntoOptionMachineOutput<profiles::MachineOutput>,
    {
        value.into_option_machine_output().map(Self::Profiles)
    }

    pub fn rapid_charge<T>(value: T) -> Option<Self>
    where
        T: IntoOptionMachineOutput<rapid_charge::MachineOutput>,
    {
        value.into_option_machine_output().map(Self::RapidCharge)
    }

    pub fn system_performance<T>(value: T) -> Option<Self>
    where
        T: IntoOptionMachineOutput<system_performance::MachineOutput>,
    {
        value
            .into_option_machine_output()
            .map(Self::SystemPerformance)
    }
}
