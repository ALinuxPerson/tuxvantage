use crate::{anyhow_with_tip, TippingAnyhowResultExt};
use ideapad::acpi_call;
use ideapad::{battery_conservation, rapid_charge, system_performance};

const ACPI_CALL_METHOD_NOT_FOUND_TIP: &str =
    "the currently used profile may not be supported for your system. use another then try again";
const ACPI_CALL_KERNEL_MODULE_NOT_LOADED_TIP: &str = "try running the following command as root to enable `acpi_call` (exclude the '#'!):\n\
# modprobe acpi_call\n\
if it says something about the module not being found in some directory, install it in your package repositories,\n\
reboot (although rebooting may not be necessary depending on your system, try it!), then perform this step again";

pub trait AcpiCallResultExt<T> {
    fn resolve_tip(self) -> anyhow_with_tip::Result<T>;
}

impl<T> AcpiCallResultExt<T> for acpi_call::Result<T> {
    fn resolve_tip(self) -> anyhow_with_tip::Result<T> {
        let tip = if let Err(acpi_call::Error::MethodNotFound { .. }) = &self {
            Some("the currently used profile may not be supported for your system. use another then try again")
        } else {
            None
        };

        self.map_err(anyhow::Error::new).maybe_tip(tip)
    }
}

pub trait AnyhowResultExt<T> {
    fn maybe_acpi_call_tip(self) -> anyhow_with_tip::Result<T>;
}

impl<T> AnyhowResultExt<T> for anyhow::Result<T> {
    fn maybe_acpi_call_tip(self) -> anyhow_with_tip::Result<T> {
        let error = self.as_ref().err();
        let tip = if let Some(error) = error {
            let error = if let Some(error) = error.downcast_ref::<acpi_call::Error>() {
                Some(error)
            } else if let Some(battery_conservation::Error::AcpiCall { error }) =
                error.downcast_ref::<battery_conservation::Error>()
            {
                Some(error)
            } else if let Some(rapid_charge::Error::AcpiCall { error }) =
                error.downcast_ref::<rapid_charge::Error>()
            {
                Some(error)
            } else if let Some(system_performance::Error::AcpiCall { error }) =
                error.downcast_ref::<system_performance::Error>()
            {
                Some(error)
            } else {
                None
            };

            if let Some(error) = error {
                match error {
                    acpi_call::Error::MethodNotFound { .. } => Some(ACPI_CALL_METHOD_NOT_FOUND_TIP),
                    acpi_call::Error::KernelModuleNotLoaded { .. } => {
                        Some(ACPI_CALL_KERNEL_MODULE_NOT_LOADED_TIP)
                    }
                    _ => None,
                }
            } else {
                None
            }
        } else {
            None
        };

        self.maybe_tip(tip)
    }
}
