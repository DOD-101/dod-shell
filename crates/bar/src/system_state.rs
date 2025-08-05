// TODO: Improve performance
use std::{
    ffi::OsString,
    fs,
    io::{Read, Write},
    net::TcpStream,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
    time::Duration,
};

use common::config::APP_CONFIG;

use hyprland::shared::HyprDataActive;
use regex::Regex;
use relm4::SharedState;
use sysinfo::{Disks, System};
use time::OffsetDateTime;

pub static SYSTEM_STATE: SharedState<SystemState> = SharedState::new();

pub fn init_update_loop() {
    relm4::spawn_blocking(|| {
        loop {
            SYSTEM_STATE.write().update();

            std::thread::sleep(Duration::from_millis(500));
        }
    });
}

// Struct  containing all of the system state
//
// Provides the [SystemState::update] method for updating all of the state.
#[derive(Debug)]
pub struct SystemState {
    sys: System,
    disks: Disks,
    data: SystemStateData,
}

impl Default for SystemState {
    fn default() -> Self {
        let sys = System::new_all();
        let mut state = Self {
            sys,
            disks: Disks::new(),
            data: SystemStateData {
                total_mem: 0,
                cpu_usage: 0.0,
                mem_usage: 0.0,
                used_mem: 0,
                time: OffsetDateTime::UNIX_EPOCH,
                workspace: 0,
                network: NetworkData::default(),
                battery: 0,
                battery_status: BatteryStatus::default(),
                disks: Arc::new([]),
                bluetooth: false,
                capslock: false,
                numlock: false,
                volume: 0.0,
            },
        };

        state.data.total_mem = state.sys.total_memory();
        state.update();

        state
    }
}

impl SystemState {
    #[allow(clippy::cast_precision_loss)]
    fn update(&mut self) {
        self.sys.refresh_all();
        self.data.cpu_usage = self.sys.global_cpu_usage();
        self.data.used_mem = self.sys.used_memory();
        self.data.mem_usage = self.data.used_mem as f32 / self.data.total_mem as f32;
        self.data.time = OffsetDateTime::now_local().expect("Failed to get time offset.");
        self.data.workspace = hyprland::data::Workspace::get_active().unwrap().id;

        self.disks.refresh(true);

        self.data.disks = self
            .disks
            .list()
            .iter()
            .map(|d| {
                let size = d.total_space();
                let free = d.available_space();
                let used = (size as f64 - free as f64) / size as f64;
                DiskData {
                    name: d.name().to_os_string(),
                    size,
                    free,
                    used,
                }
            })
            .collect();

        if let Some(bat) = &APP_CONFIG.battery {
            let battery_path = PathBuf::from("/sys/class/power_supply/").join(bat);

            self.data.battery =
                fs::read_to_string(battery_path.join("capacity")).map_or(0, |percentage| {
                    percentage
                        .trim()
                        .parse()
                        .expect("Invalid battery percentage in /sys")
                });

            // TODO: Could need some error handling here
            // TODO: This causes a hard to understand panic if the config isn't set correctly
            self.data.battery_status = match fs::read_to_string(battery_path.join("status"))
                .expect("Failed to read battery status")
                .trim()
            {
                "Charging" => BatteryStatus::Charging,
                "Discharging" => BatteryStatus::Discharging,
                status => {
                    log::warn!("Unknown battery status: {status}");
                    BatteryStatus::Unknown
                }
            }
        }

        // TODO: I don't think I need to explain why this is bad. A native rust solution (aka. lib)
        // would be much better
        self.data.network = NetworkData {
            internet: NetworkData::test_connection().unwrap_or_default(),
            name: Command::new("iwgetid")
                .arg("-r")
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                .ok()
                // If the network name is just empty make it [None]
                .and_then(|name| if name.is_empty() { None } else { Some(name) }),
            connection_strengh: {
                Command::new("iwconfig")
                    .output()
                    .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                    .ok()
                    .and_then(|info| {
                        info.lines()
                            .find(|l| l.contains("Link Quality"))
                            // Link Quality=56/70  Signal level=-54 dBm
                            .and_then(|s| s.split('=').nth(1))
                            // 54/70 Signal Level
                            .and_then(|s| {
                                s.split_once(' ').and_then(|parts| {
                                    // 54/70
                                    parts.0.split_once('/').map(|num_parts| {
                                        num_parts.0.parse::<f32>().unwrap()
                                            / num_parts.1.parse::<f32>().unwrap()
                                    })
                                })
                            })
                    })
                    .unwrap_or_default()
            },
        };

        self.data.bluetooth = Command::new("bluetoothctl")
            .arg("info")
            .stdout(Stdio::null())
            .stdin(Stdio::null())
            .status()
            .is_ok_and(|v| v.success());

        (self.data.capslock, self.data.numlock) = get_key_states().unwrap_or_default();

        self.data.volume = Command::new("wpctl")
            .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
            .output()
            .ok()
            .and_then(|o| {
                let output = String::from_utf8_lossy(&o.stdout).to_string();
                let parts = output.split_once(' ').expect("Volume format invalid.");

                if parts.1.contains("MUTED") {
                    return Some(-1.0);
                }

                parts.1.trim().parse::<f32>().ok()
            })
            .unwrap_or_default();

        log::trace!("State updated");
    }

    pub fn get_data(&self) -> &SystemStateData {
        &self.data
    }
}

#[derive(Debug, Clone)]
pub struct SystemStateData {
    pub total_mem: u64,
    pub cpu_usage: f32,
    pub mem_usage: f32,
    pub used_mem: u64,
    pub time: OffsetDateTime,
    pub workspace: i32,
    pub network: NetworkData,
    /// Battery Charge (in %)
    pub battery: u8,
    /// Battery Status
    pub battery_status: BatteryStatus,
    pub disks: Arc<[DiskData]>,
    /// If there are currently any devices connected via Bluetooth
    pub bluetooth: bool,
    /// If capslock is active
    pub capslock: bool,
    /// If numlock is active
    pub numlock: bool,
    /// Volume of the default audio output
    pub volume: f32,
}

#[derive(Debug, Clone)]
pub struct DiskData {
    /// The name of the disk
    pub name: OsString,
    /// The total space on the disk (in bytes)
    pub size: u64,
    /// How much space is free on the disk (in bytes)
    pub free: u64,
    /// How much space is used on the disk (in % of size)
    pub used: f64,
}

#[derive(Default, Debug, Clone)]
pub struct NetworkData {
    pub internet: bool,
    pub name: Option<String>,
    pub connection_strengh: f32,
}

impl NetworkData {
    // TODO: This seems to be a performance bottleneck
    fn test_connection() -> std::io::Result<bool> {
        let mut stream =
            TcpStream::connect("52.142.124.215:80").inspect_err(|e| println!("Err here: {e}"))?; // ipv4 address for duck.com

        stream.write_all(&[1])?;
        stream.read_exact(&mut [0; 128])?;
        Ok(true)
    }
}

impl NetworkData {
    pub fn wireless(&self) -> bool {
        self.name.is_some()
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum BatteryStatus {
    Discharging,
    Charging,
    #[default]
    Unknown,
}

fn read_brightness(path: &str) -> Result<u32, std::io::Error> {
    let content = fs::read_to_string(path)?;
    content
        .trim()
        .parse::<u32>()
        .map_err(|_| std::io::ErrorKind::Other.into())
}

fn get_key_states() -> Result<(bool, bool), std::io::Error> {
    let capslock_pattern = Regex::new(r"input\d+::capslock").unwrap();
    let numlock_pattern = Regex::new(r"input\d+::numlock").unwrap();

    let led_dir = Path::new("/sys/class/leds");
    let entries = fs::read_dir(led_dir)?;

    let mut capslock_brightness_sum = 0;
    let mut numlock_brightness_sum = 0;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if let Some(file_name) = path.file_name() {
            let file_name_str = file_name.to_string_lossy();
            let brightness_path = path.join("brightness");

            // Check if the directory name matches the Caps Lock or Num Lock pattern
            if capslock_pattern.is_match(&file_name_str) && brightness_path.exists() {
                if let Ok(brightness) = read_brightness(&brightness_path.display().to_string()) {
                    capslock_brightness_sum += brightness;
                }
            } else if numlock_pattern.is_match(&file_name_str) && brightness_path.exists() {
                if let Ok(brightness) = read_brightness(&brightness_path.display().to_string()) {
                    numlock_brightness_sum += brightness;
                }
            }
        }
    }

    Ok((capslock_brightness_sum > 0, numlock_brightness_sum > 0))
}
