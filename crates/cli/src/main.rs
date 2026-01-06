//! CLI to go along with the dod-shell
//!
//! This CLI is used to interact with the different components of the shell.
use clap::{Parser, Subcommand, ValueEnum};
use common::config;
use prettytable::{Table, row};
use strum::{Display, IntoEnumIterator};
use sysinfo::{Process, ProcessRefreshKind, System};

use std::{
    ffi::OsStr,
    fmt::Write,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

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
    /// Generate the default config. See [`generate_config`]
    #[command(about = "Generate the default config")]
    GenerateConfig {
        /// Generate only `layouts.json.schema`
        #[arg(short, long)]
        schema_only: bool,
        /// Overwrite files if they already exist
        #[arg(short, long)]
        overwrite: bool,
        /// Where to put the generated files
        #[arg(default_value=common::CONFIG_PATH.clone().into_os_string())]
        path: PathBuf,
    },
}

#[derive(Clone, ValueEnum, Debug, Display)]
/// The different components of the shell
#[strum(serialize_all = "lowercase")]
enum Component {
    /// The launcher component. See `launcher` crate.
    Launcher,
    /// The bar component. See `bar` crate.
    Bar,
    /// The daemon component. See `deamon` crate.
    Daemon,
    /// The osk component. See`osk` crate.
    Osk,
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
        log::error!("Failed to launch {component}. Error: {e}");
    }
}

/// Wrapper type to indicate Bytes
struct Bytes(u64);

impl Bytes {
    /// Convert to kilobytes
    const fn to_kb(&self) -> u64 {
        self.0 / 1_048_576 // 1024 * 1024
    }
}

/// Struct for holding information about a process
///
/// Fields are mostly gathered from [`sysinfo::Process`]
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

        Self {
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
///
/// And the totals of Memory and CPU.
fn list() {
    let mut sys = System::new();

    sys.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .with_memory()
            .with_exe(sysinfo::UpdateKind::OnlyIfNotSet),
    );

    let mut processes: Vec<ProcessInfo> = sys
        .processes_by_name(OsStr::new("dod-shell"))
        .map(ProcessInfo::from)
        .collect();

    processes.sort_unstable_by(|a, b| a.name.cmp(&b.name));

    let mut table = Table::new();

    table.add_row(row!["Name", "Memory (MB)", "CPU (%)", "PID"]);

    let mut cpu_total: f32 = 0.0;
    let mut memory_total = 0;

    for process in processes {
        let cpu_usage = process.cpu_usage.round();
        cpu_total += cpu_usage;
        let mem_usage = process.mem_usage.to_kb();
        memory_total += mem_usage;

        table.add_row(row![
            process.name.unwrap_or_default(),
            mem_usage,
            cpu_usage,
            process.pid,
        ]);
    }

    table.printstd();

    println!("\nTotals:");
    println!("        Memory: {memory_total}MB");
    println!("        CPU: {cpu_total}%");
}

/// Check which parts of the shell are installed
///
/// If the component is installed also returns the path to the binary.
///
/// ## Output
///
/// For output format see [`print_installed`]. Each result is printed on a new line.
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
            .arg("dod-shell-".to_string() + &cmp)
            .output();

        print_installed(
            &cmp,
            result.ok().and_then(|r| {
                if r.stdout.is_empty() {
                    None
                } else {
                    Some(String::from_utf8(r.stdout).unwrap())
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
    let log_term_err = |e: term::Error| log::error!("Failed to set term color. Error: {e}");
    let mut t = term::stdout().unwrap();

    write!(t, "{component}: ").expect("Failed to write to stdout.");

    if let Some(path) = path {
        t.fg(term::color::GREEN).unwrap_or_else(log_term_err);

        write!(t, "Yes @ {path}").expect("Failed to write to stdout.");
    } else {
        t.fg(term::color::RED).unwrap_or_else(log_term_err);
        writeln!(t, "No").expect("Failed to write to stdout.");
    }

    t.reset().unwrap_or_else(log_term_err);
}

/// Generates the default config
///
/// The files will be written to the passed `path`.
///
/// If `schema_only` then only `layouts.schema.json` will be generated and written.
///
/// ## Output
///
/// None other than errors.
fn generate_config(schema_only: bool, path: &Path, overwrite: bool) {
    if let Err(err) = fs::create_dir_all(path) {
        log::error!("Failed to create dir \"{}\": {err}", path.to_string_lossy());
    }

    let schema_path = path.join("layouts.schema.json");
    let schema = serde_json::to_string_pretty(&schemars::schema_for!(config::layouts::Layouts))
        .expect("Should never fail to serialize json schema as json.");

    write_file(schema_path, schema, overwrite);

    if schema_only {
        return;
    }

    let layouts_path = path.join("layouts.json");
    let mut layouts = serde_json::to_string_pretty(&config::layouts::Layouts::default())
        .expect("Layouts should always be valid json.");

    layouts.insert_str(2, "  \"$schema\": \"./layouts.schema.json\",\n");

    write_file(layouts_path, layouts, overwrite);

    let config_path = path.join("config.toml");
    let config = toml::to_string_pretty(&config::Config::default())
        .expect("Config should always be valid toml.");

    write_file(config_path, config, overwrite);

    let css_path = path.join("style.scss");
    let css: String =
        "/* All available css classes */\n* { all: unset; } // recommended\n".to_string();

    let css = common::css::Class::iter().fold(css, |mut css, class| {
        writeln!(css, ".{class} {{}}").expect("Should never fail to write to String");

        css
    });

    write_file(css_path, css, overwrite);
}

/// Wrapper function around [`fs::write`]
///
/// 1. Logs the returned error
///
/// 2. Only overwrites files if passed with `overwrite == true`
fn write_file<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C, overwrite: bool) {
    let path = path.as_ref();

    match path.try_exists() {
        Ok(true) if !overwrite => {
            log::warn!("File \"{}\" already exists.", path.to_string_lossy());

            log::info!("Pass -o to overwrite");
            return;
        }
        Ok(_) => {}
        Err(err) => {
            log::error!("Error: {err}");
            log::error!(
                "Could not determine if \"{}\" already exists.",
                path.to_string_lossy()
            );

            if !overwrite {
                return;
            }
        }
    }

    if let Err(e) = fs::write(path, contents) {
        log::error!("Failed to write \"{}\": {e}", path.to_string_lossy());
    }
}

fn main() {
    let args = Cli::parse();
    simple_logger::SimpleLogger::new().env().init().unwrap();

    match args.action {
        Action::Launch { component } => launch(&component.to_string()),
        Action::List => list(),
        Action::Installed { component } => installed(component),
        Action::GenerateConfig {
            schema_only,
            path,
            overwrite,
        } => generate_config(schema_only, &path, overwrite),
    }
}
