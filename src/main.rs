#[macro_use]
extern crate serde;

#[macro_use]
mod macros;

mod anyhow_with_tip;
mod app;
mod args;
mod config;
mod context;
mod ext;
mod log;
mod machine;
mod project_paths;
mod utils;
mod verbose;

use crate::anyhow_with_tip::TippingAnyhowResultExt;
use crate::args::TuxVantageAction;
use crate::machine::Machine;
use crate::utils::not;
use anyhow::Context as AnyhowContext;
use args::*;
use context::Context;
use ideapad::Profile;
use itertools::Itertools;
use owo_colors::OwoColorize;
use parking_lot::RwLockWriteGuard;
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{env, io, process, thread};
use tap::Pipe;
use tokio::sync::broadcast::error::RecvError;
use try_drop::drop_strategies::{BroadcastDropStrategy, PanicDropStrategy};

fn main() {
    static MACHINE: AtomicBool = AtomicBool::new(false);
    static BACKTRACE: AtomicBool = AtomicBool::new(false);
    static PANIC: AtomicBool = AtomicBool::new(false);

    color_backtrace::install();

    fn inner() -> anyhow_with_tip::Result<Option<app::MachineOutput>> {
        let args = args::parse();
        verbose::set(args.verbose);
        debug!("hello world!");

        let machine = args.machine.unwrap_or_default().get();
        debug!("set global machine to {machine}");
        MACHINE.store(machine, Ordering::SeqCst);

        debug!("set global panic to {}", args.panic);
        PANIC.store(args.panic, Ordering::SeqCst);

        debug!("initialize project paths");
        project_paths::initialize().context("failed to initialize project paths")?;

        debug!("initialize config");
        let result = config::initialize().context("failed to initialize config");
        let errors = {
            if result.is_err() {
                debug!(
                    "failed to initialize configuration, configuring backtrace before bailing out"
                );
                args.backtrace.configure();
                BACKTRACE.store(args.backtrace.errors, Ordering::SeqCst);
            }

            result?
        };
        let machine = config::machine();

        if !errors.is_empty() && not(machine) {
            warn!(
                "recoverable errors occurred during config initialization. see below for details"
            );

            for error in errors {
                warn!("{:#}", error);
            }
        }

        // operating with the config needs to be in a scope so that the guard will get dropped
        // since we're operating with an `RwLock` otherwise we'll get a deadlock
        {
            let mut config = config::write();

            if !args.skip_consistency_checks {
                debug!("starting consistency checks");

                let current_exe = env::current_exe()
                    .context("failed to get current executable location of tuxvantage")?;
                debug!("current exe location is '{}'", current_exe.display());

                match config.consistency.last_exe.as_ref() {
                    Some(last_exe) => {
                        if config.consistency.regulator_service_installed
                            && (last_exe != &current_exe)
                        {
                            warn!(
                                "the last executable used to install the battery conservation regulator service, {}, differs from the current executable location \
running this program, {}.\n\n\
this behavior may be undesirable, since the battery conservation regulator service may fail to run with a no such file or directory error.\n\
if this is the case, try running {} again.",
                                last_exe.display().bold(),
                                current_exe.display().bold(),
                                "tuxvantage battery-conservation regulate -I".bold()
                            )
                        }
                    }
                    None => {
                        debug!("no last exe found, setting it to current exe");
                        config.consistency.last_exe = Some(current_exe);
                    }
                }
            }

            debug!("setup config overrides from arguments");
            config.tuxvantage.overrides.machine = args.machine;
            config.tuxvantage.overrides.profile = args.profile;
            config.tuxvantage.overrides.handlers.default = args.handler.map(|handler| handler.0);
            config.tuxvantage.overrides.backtrace = args.backtrace;
            config.tuxvantage.overrides.panic = args.panic;

            debug!("configure backtrace");
            let backtrace = config.tuxvantage.backtrace();
            backtrace.configure();
            BACKTRACE.store(backtrace.errors, Ordering::SeqCst);

            debug!("set up panic toggle");
            PANIC.store(config.tuxvantage.panic(), Ordering::SeqCst);

            // downgrading the guard to read-only does not help with the deadlock
            let config = RwLockWriteGuard::downgrade(config);

            if !matches!(args.action, TuxVantageAction::Profiles(_)) {
                debug!("initializing ideapad");
                let profile = match config.default_profile() {
                    Some(profile) => {
                        debug!("config has default profile");
                        profile.context("failed to get the default profile")?
                    }
                    None => {
                        debug!("no default profile is used, using search path from detected profiles with built ins");
                        let search_path = config
                            .profiles
                            .with_built_ins()
                            .map(|profile| profile.get().deref().clone());

                        let result = Profile::find_with_search_path(search_path);
                        let tip = if let Err(ideapad::profile::Error::Io { error }) = &result {
                            if error.kind() == io::ErrorKind::PermissionDenied {
                                Some("this program tries to identify the product of your machine which requires root privileges, so try running this program as root")
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        result
                            .context("failed to initialize ideapad")
                            .maybe_tip(tip)?
                    }
                };

                debug!("setup up drop strategy");
                let (fallible_drop_strategy, mut receiver) = BroadcastDropStrategy::new(16);
                let context = Context::new_with_strategies(
                    profile,
                    fallible_drop_strategy,
                    PanicDropStrategy::default(),
                );

                thread::spawn(move || {
                    debug!("start drop strategy receiver thread");

                    loop {
                        let recv = receiver.recv();

                        match recv {
                            Ok(error) => {
                                error!("failed to drop something: {error}");
                                debug!("debug representation of the drop error:\n {error:#?}")
                            }
                            Err(RecvError::Lagged(count)) => {
                                warn!("drop strategy receiver thread lagged too far behind: {count} skipped messages");
                                warn!("continuing");
                                continue;
                            }
                            Err(RecvError::Closed) => break,
                        }
                    }
                });

                context::initialize(context);

                debug!("ideapad initialized");
            }
        }

        debug!("begin to run action");
        match args.action {
            TuxVantageAction::BatteryConservation(battery_conservation) => {
                match battery_conservation {
                    TuxVantageBatteryConservation::Enabled => app::battery_conservation::enabled()
                        .map(app::MachineOutput::battery_conservation),
                    TuxVantageBatteryConservation::Disabled => {
                        app::battery_conservation::disabled()
                            .map(app::MachineOutput::battery_conservation)
                    }
                    TuxVantageBatteryConservation::Enable { handler } => {
                        app::battery_conservation::enable(handler)
                            .map(app::MachineOutput::battery_conservation)
                    }
                    TuxVantageBatteryConservation::Disable => app::battery_conservation::disable()
                        .map(app::MachineOutput::battery_conservation),
                    TuxVantageBatteryConservation::Regulate {
                        threshold,
                        cooldown,
                        infallible,
                        matches,
                        install,
                    } => app::battery_conservation::regulate(
                        threshold, cooldown, infallible, matches, install,
                    )
                    .map(app::MachineOutput::battery_conservation),
                }
            }
            TuxVantageAction::SystemPerformance(system_performance) => match system_performance {
                TuxVantageSystemPerformance::Get => {
                    app::system_performance::get().map(app::MachineOutput::system_performance)
                }
                TuxVantageSystemPerformance::Set { mode } => {
                    app::system_performance::set(mode).map(app::MachineOutput::system_performance)
                }
            },
            TuxVantageAction::RapidCharge(rapid_charge) => match rapid_charge {
                TuxVantageRapidCharge::Enabled => {
                    app::rapid_charge::enabled().map(app::MachineOutput::rapid_charge)
                }
                TuxVantageRapidCharge::Disabled => {
                    app::rapid_charge::disabled().map(app::MachineOutput::rapid_charge)
                }
                TuxVantageRapidCharge::Enable { handler } => {
                    app::rapid_charge::enable(handler).map(app::MachineOutput::rapid_charge)
                }
                TuxVantageRapidCharge::Disable => {
                    app::rapid_charge::disable().map(app::MachineOutput::rapid_charge)
                }
            },
            TuxVantageAction::Profiles(profiles) => match profiles {
                TuxVantageProfiles::Get { name } => app::profiles::get(name)
                    .map(app::MachineOutput::profiles)
                    .no_tip(),
                TuxVantageProfiles::GetDefault => app::profiles::get_default()
                    .map(app::MachineOutput::profiles)
                    .no_tip(),
                TuxVantageProfiles::Set {
                    name,
                    contents,
                    create_new,
                } => app::profiles::set(name, contents, create_new)
                    .map(app::MachineOutput::profiles)
                    .no_tip(),
                TuxVantageProfiles::SetDefault { name } => app::profiles::set_default(name)
                    .map(app::MachineOutput::profiles)
                    .no_tip(),
                TuxVantageProfiles::Remove { name } => {
                    app::profiles::remove(name).map(app::MachineOutput::profiles)
                }
                TuxVantageProfiles::Json {
                    name,
                    generate_on_error,
                    pretty,
                } => app::profiles::json(name, generate_on_error, pretty)
                    .map(app::MachineOutput::profiles)
                    .no_tip(),
            },
        }
    }

    let result = inner();

    if result.is_ok() {
        debug!("main function was ok")
    }

    if result.is_err() {
        debug!("main function returned an error")
    }

    let machine = MACHINE.load(Ordering::SeqCst);
    debug!("after main function, machine is {machine}");

    let backtrace = BACKTRACE.load(Ordering::SeqCst);
    let panic = PANIC.load(Ordering::SeqCst);

    if panic {
        debug!("was told to panic, so panicking now (if any error occurred)");
        result.as_ref().unwrap();
    }

    match result {
        Ok(machine_output) => {
            if machine {
                let output = Machine::success(machine_output)
                    .pipe_ref(serde_json::to_string)
                    .expect("failed to serialize machine output");

                println!("{}", output);
            }

            0
        }
        Err(error) => {
            debug!("debug representation of the main error:\n {error:#?}");
            if machine {
                let output = Machine::<()>::failure(error)
                    .pipe_ref(serde_json::to_string)
                    .expect("failed to serialize machine output");

                println!("{}", output);
            } else {
                let mut message = format!("{}\n", error.source.bold());
                let chain = error
                    .source
                    .chain()
                    .skip(1)
                    .map(ToString::to_string)
                    .unique();

                for error in chain {
                    message.push_str(&format!("    caused by {}\n", error.italic()));
                }

                error!("{}", message);

                if let Some(tip) = error.tip {
                    tip!("{}", tip);
                }

                if backtrace {
                    info!(
                        "a backtrace was provided alongside the error:\n{}",
                        error.source.backtrace()
                    );
                }
            }

            1
        }
    }
    .pipe(process::exit)
}
