use clap::{Parser, Subcommand, ValueEnum};
use prettytable::{Table, row};
use sysinfo::{Process, ProcessRefreshKind, System};

use core::fmt;
use std::{ffi::OsStr, process::Command};

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand, Clone, Debug)]
enum Action {
    /// Launch a specific part of the shell
    Launch { component: Component },
    /// List information about all running parts of the shell
    List,
    /// Check what parts of the shell are installed
    Installed {
        /// Check for one specific part
        component: Option<Component>,
    },
}

#[derive(Clone, ValueEnum, Debug)]
enum Component {
    Launcher,
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

fn launch(component: &str) {
    if cfg!(debug_assertions) {
        if let Err(e) = Command::new("cargo").args(["run", "-p", component]).spawn() {
            log::error!("Failed to launch {}. Error: {}", component, e);
        }
        return;
    }

    if let Err(e) = Command::new("dod-shell-".to_string() + component).spawn() {
        log::error!("Failed to launch {}. Error: {}", component, e);
    };
}

struct Bytes(u64);

impl Bytes {
    fn to_kb(&self) -> u64 {
        self.0 / 1048576 // 1024 * 1024
    }
}

struct ProcessInfo {
    name: Option<String>,
    mem_usage: Bytes,
    cpu_usage: f32,
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
