//! CLI to go along with the dod-shell
//!
//! This CLI is used to interact with the different components of the shell.
use clap::{Parser, Subcommand, ValueEnum};
use prettytable::{Table, row};
use sysinfo::{Process, ProcessRefreshKind, System};

use core::fmt;
use std::{ffi::OsStr, process::Command};

#[derive(Parser, Debug)]
/// The CLI for the dod-shell
///
/// All interaction with the different components of the shell should be done through this CLI.
/// Although it is possible to launch the components directly.
struct Cli {
    /// The [Action] to perform
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand, Clone, Debug)]
/// Different things the CLI can do
enum Action {
    /// Launch a specific part of the shell. See [launch]
    #[command(about = "Launch a specific part of the shell")]
    Launch {
        /// The component to launch
        component: Component,
    },
    /// List information about all running parts of the shell. See [list]
    #[command(about = "List information about all running parts of the shell")]
    List,
    /// Check what parts of the shell are installed. See [installed]
    #[command(about = "Check what parts of the shell are installed")]
    Installed {
        /// Check for one specific part
        component: Option<Component>,
    },
}

#[derive(Clone, ValueEnum, Debug)]
/// The different components of the shell
enum Component {
    /// The launcher component. See `launcher` crate.
    Launcher,
    /// The bar component. See `bar` crate.
    Bar,
}

impl fmt::Display for Component {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: Macro?
        let str = match self {
            Self::Launcher => "launcher",
            Self::Bar => "bar",
        };

        write!(f, "{}", str)
    }
}

/// Launch a specific component of the shell
///
/// ## Output
///
/// None of it's own, but the output of the launched component is passed.
fn launch(component: &str) {
    let cmd = if cfg!(debug_assertions) {
        Command::new("cargo").args(["run", "-p", component]).spawn()
    } else {
        Command::new("dod-shell-".to_string() + component).spawn()
    };

    if let Err(e) = cmd {
        log::error!("Failed to launch {}. Error: {}", component, e);
    };
}

/// Wrapper type to indicate Bytes
// NOTE: Do we even need this type?
struct Bytes(u64);

impl Bytes {
    /// Convert to kilobytes
    fn to_kb(&self) -> u64 {
        self.0 / 1048576 // 1024 * 1024
    }
}

/// Struct for holding information about a process
///
/// Fields are mostly gathered from [sysinfo::Process]
// NOTE: Do we even need this type?
struct ProcessInfo {
    /// The name of the process
    name: Option<String>,
    /// The memory usage of the process
    mem_usage: Bytes,
    /// The CPU usage of the process
    cpu_usage: f32,
    /// The PID of the process
    pid: u32,
}

impl From<&Process> for ProcessInfo {
    fn from(value: &Process) -> Self {
        let name = value.exe().map(|e| {
            e.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });

        let mem_usage = Bytes(value.memory());

        let cpu_usage = value.cpu_usage();

        let pid = value.pid().as_u32();

        ProcessInfo {
            name,
            mem_usage,
            cpu_usage,
            pid,
        }
    }
}

/// List all running components of the shell
///
/// ## Output
///
/// The output is a table with the following columns:
/// - Name: The name of the process
/// - Memory (MB): The memory usage of the process
/// - CPU (%): The CPU usage of the process
/// - PID: The PID of the process
// TODO: Add filtering to prevent the same component showing up multiple times
fn list() {
    let mut sys = System::new();

    sys.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .with_memory()
            .with_exe(sysinfo::UpdateKind::OnlyIfNotSet),
    );

    let processes: Vec<ProcessInfo> = sys
        .processes_by_name(OsStr::new("dod-shell"))
        .map(ProcessInfo::from)
        .collect();

    let mut table = Table::new();

    table.add_row(row!["Name", "Memory (MB)", "CPU (%)", "PID"]);

    for process in processes {
        table.add_row(row![
            process.name.unwrap_or_default(),
            process.mem_usage.to_kb(),
            process.cpu_usage.round(),
            process.pid,
        ]);
    }

    table.printstd();
}

/// Check which parts of the shell are installed
///
/// If the component is installed also returns the path to the binary.
///
/// ## Output
///
/// For output format see [print_installed]. Each result is printed on a new line.
fn installed(component: Option<Component>) {
    let components: Vec<String> = component.map_or_else(
        || {
            Component::value_variants()
                .iter()
                .map(Component::to_string)
                .collect::<Vec<String>>()
        },
        |c| vec![c.to_string()],
    );

    for cmp in components {
        let result = Command::new("which")
            .arg("dod-shell-".to_string() + &cmp.to_string())
            .output();

        print_installed(
            &cmp.to_string(),
            result.ok().and_then(|r| {
                if !r.stdout.is_empty() {
                    Some(String::from_utf8(r.stdout).unwrap())
                } else {
                    None
                }
            }),
        );
    }
}

/// Helper function to print individual results of [installed]
///
/// ## Output
///
/// If installed:
///
/// name: <span style="color:green;">Yes</span> @ /path/to/binary
///
/// If not installed:
///
/// name: <span style="color:red;">No</span>
fn print_installed(component: &str, path: Option<String>) {
    let log_term_err = |e: term::Error| log::error!("Failed to set term color. Error: {}", e);
    let mut t = term::stdout().unwrap();

    write!(t, "{}: ", component).expect("Failed to write to stdout.");

    if let Some(path) = path {
        t.fg(term::color::GREEN).unwrap_or_else(log_term_err);

        write!(t, "Yes @ {}", path).expect("Failed to write to stdout.")
    } else {
        t.fg(term::color::RED).unwrap_or_else(log_term_err);
        writeln!(t, "No").expect("Failed to write to stdout.");
    };

    t.reset().unwrap_or_else(log_term_err);
}

fn main() {
    let args = Cli::parse();
    simple_logger::SimpleLogger::new().env().init().unwrap();

    match args.action {
        Action::Launch { component } => match component {
            Component::Launcher => launch("launcher"),
            Component::Bar => launch("bar"),
        },
        Action::List => list(),
        Action::Installed { component } => installed(component),
    };
}
