//! This module contains all the code required for getting the "State" (aka. Information) from
//! other services
//!
//! The main type is [``SystemState``]
use std::{
    collections::HashMap,
    convert::TryInto,
    ffi::OsString,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
    time::Duration,
};

use common::{config::APP_CONFIG, types::Percentage};

use alsa::{
    Mixer,
    mixer::{SelemChannelId, SelemId},
};
use hyprland::shared::HyprDataActive;
use regex::Regex;
use relm4::SharedState;
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};
use time::OffsetDateTime;
use tokio::fs;
use zbus::{
    Proxy,
    fdo::PropertiesProxy,
    names::InterfaceName,
    zvariant::{Array, ObjectPath, OwnedObjectPath, OwnedValue, Value},
};

/// Dbus service name for `NetworkManager` used by [``SystemState::network``]
const NM_SERVICE_NAME: &str = "org.freedesktop.NetworkManager";

/// [``Regex``] used by [``SystemState::key_states``]
static CAPSLOCK_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"input\d+::capslock").unwrap());
/// [``Regex``] used by [``SystemState::key_states``]
static NUMLOCK_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"input\d+::numlock").unwrap());

/// [``SystemState``] instance to be used throughout rest of the crate
pub static SYSTEM_STATE: SharedState<SystemState> = SharedState::new();

/// Starts background thread to periodically update [``SYSTEM_STATE``]
///
/// If this is not run before starting the main [``relm4::RelmApp``] the state will never be
/// updated.
///
/// Also logs (at the trace level) the time it took to run [``SystemState::update``].
pub fn init_update_loop() {
    #[allow(clippy::redundant_closure_call)]
    relm4::spawn_local((async || {
        let mut update_interval = tokio::time::interval(Duration::from_millis(500));

        loop {
            let start = tokio::time::Instant::now();
            let mut lock = SYSTEM_STATE.write();

            lock.update().await;

            let end = tokio::time::Instant::now();

            let delta = (end - start).as_millis();

            log::trace!("State updated. Took {delta}ms");

            update_interval.tick().await;
        }
    })());
}

/// All of the State (aka. Information) gathered from the system
///
/// Provides the [``Self::update``] method for updating said state.
///
/// Internally holds [``SystemStateData``] to actually hold all of the data.
///
/// Other than the data itself it contains Objects needed to update parts of the state, which
/// shouldn't be re-created each time [``Self::update``] is run due to performance reasons
#[derive(Debug)]
pub struct SystemState {
    /// Used in [``Self::update``]
    sys: System,
    /// Used in [``Self::update``]
    disks: Disks,
    /// The actual data
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
                cpu_usage: Percentage::default(),
                mem_usage: Percentage::default(),
                used_mem: 0,
                time: OffsetDateTime::UNIX_EPOCH,
                workspace: 0,
                network: ConnectionData::default(),
                battery: Percentage::default(),
                battery_status: BatteryStatus::default(),
                disks: Arc::new([]),
                bluetooth: false,
                capslock: false,
                numlock: false,
                volume: Percentage::default(),
            },
        };

        state.data.total_mem = state.sys.total_memory();

        // TODO: remove this
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(state.update());

        state
    }
}

impl SystemState {
    /// Used for updating the state
    #[allow(clippy::cast_precision_loss)]
    #[allow(clippy::cast_possible_truncation)]
    async fn update(&mut self) {
        self.sys.refresh_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
                .with_memory(MemoryRefreshKind::nothing().with_ram()),
        );
        self.data.cpu_usage = self.sys.global_cpu_usage().into();
        self.data.used_mem = self.sys.used_memory();
        self.data.mem_usage = (self.data.used_mem as f32 / self.data.total_mem as f32).into();
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
                let used = (((size as f64 - free as f64) / size as f64) as f32).into();
                DiskData {
                    name: d.name().to_os_string(),
                    size,
                    free,
                    used,
                }
            })
            .collect();

        let (bluetooth, network, key_states, battery_data) = tokio::join!(
            self.bluetooth(),
            self.network(),
            SystemState::key_states(),
            SystemState::battery()
        );

        // NOTE: Might be nice to use a macro here

        self.data.bluetooth = bluetooth
            .inspect_err(|e| log::error!("Failed to update bluetooth information: {e}"))
            .unwrap_or_default();

        self.data.network = network
            .inspect_err(|e| log::error!("Failed to update network information: {e}"))
            .unwrap_or_default();

        (self.data.capslock, self.data.numlock) = key_states
            .inspect_err(|e| log::error!("Failed to update key state information: {e}"))
            .unwrap_or_default();

        if let Some(battery_data) = battery_data {
            (self.data.battery, self.data.battery_status) = battery_data
                .inspect_err(|e| log::error!("Failed to update battery information: {e}"))
                .unwrap_or_default();
        }

        self.data.volume = SystemState::volume()
            .inspect_err(|e| log::error!("Failed to update volume information: {e}"))
            .unwrap_or_default();
    }

    /// Get's the internal data
    ///
    /// ## ⚠️ Warning ⚠️
    ///
    /// Will not update data before returning it
    pub fn get_data(&self) -> &SystemStateData {
        &self.data
    }

    /// Checks if any devices are connected via bluetooth
    ///
    /// Used in [``Self::update``]
    async fn bluetooth(&self) -> zbus::Result<bool> {
        let connection = zbus::Connection::system().await?;
        // Create a proxy to interact with BlueZ's ObjectManager interface
        // ObjectManager provides a way to discover all available objects and their interfaces
        // BlueZ: https://git.kernel.org/pub/scm/bluetooth/bluez.git/tree/
        let bluez_proxy = zbus::Proxy::new(
            &connection,
            "org.bluez",
            "/",
            "org.freedesktop.DBus.ObjectManager",
        )
        .await?;

        // Call GetManagedObjects to retrieve all BlueZ objects (adapters, devices, etc.)
        // This returns a complex nested structure containing all objects and their properties
        let reply = bluez_proxy
            .call_method("GetManagedObjects", &())
            .await?
            .body();

        // Deserialize the D-Bus message body into a structured format
        // Type signature: Dict<ObjectPath, Dict<InterfaceName, Dict<PropertyName, Variant>>>
        let managed_objects: HashMap<OwnedObjectPath, HashMap<String, HashMap<String, Value<'_>>>> =
            reply.deserialize()?;

        // Iterate through all managed objects
        for interfaces in managed_objects.values() {
            // Check if this object implements the Device1 interface
            if let Some(device_props) = interfaces.get("org.bluez.Device1") {
                // Check if the device is connected
                if let Some(connected_value) = device_props.get("Connected") {
                    if let Ok(is_connected) = bool::try_from(connected_value) {
                        if is_connected {
                            return Ok(true);
                        }
                    }
                }
            }
        }

        Ok(false)
    }

    /// Get's information about the battery, if one is set in the shell [Config](``common::config::Config``)
    ///
    /// Used in [``Self::update``]
    async fn battery() -> Option<std::io::Result<(Percentage, BatteryStatus)>> {
        if let Some(bat) = &APP_CONFIG.bar.battery {
            let battery_path = PathBuf::from("/sys/class/power_supply/").join(bat);

            let percentage: std::io::Result<u8> = fs::read_to_string(battery_path.join("capacity"))
                .await
                .map(|s| {
                    s.trim()
                        .parse()
                        .expect("Value in capacity file should be number")
                });

            let status: std::io::Result<BatteryStatus> =
                fs::read_to_string(battery_path.join("status"))
                    .await
                    .map(|s| s.trim().into());

            return match (percentage, status) {
                (Ok(p), Ok(s)) => Some(Ok((p.into(), s))),
                (Err(e), _) | (_, Err(e)) => Some(Err(e)),
            };
        }

        None
    }

    /// Gathers information about the current internet connection
    ///
    /// Used in [``Self::update``]
    async fn network(&self) -> Result<ConnectionData, Box<dyn std::error::Error>> {
        let connection = zbus::Connection::system().await?;
        let nm_iface = InterfaceName::from_str_unchecked(NM_SERVICE_NAME);
        let nm_proxy = Proxy::new(
            &connection,
            NM_SERVICE_NAME,
            "/org/freedesktop/NetworkManager",
            nm_iface,
        )
        .await?;

        let state: u32 = nm_proxy.call("state", &()).await?;

        // NMState = 70 means connected to the internet
        // See: https://networkmanager.dev/docs/api/latest/nm-dbus-types.html#NMState
        if state != 70 {
            return Ok(ConnectionData::None);
        }

        let devices: Vec<ObjectPath> =
            nm_proxy
                .get_property::<OwnedValue>("Devices")
                .await
                .map(|devices| {
                    devices
                        .try_into()
                        // See: https://networkmanager.dev/docs/api/latest/gdbus-org.freedesktop.NetworkManager.html#gdbus-property-org-freedesktop-NetworkManager.Devices
                        .expect("Devices property should be a list of ObjectPaths")
                })?;

        for d in devices {
            let device_proxy = PropertiesProxy::new(&connection, NM_SERVICE_NAME, &d).await?;

            let device_iface =
                InterfaceName::from_str_unchecked("org.freedesktop.NetworkManager.Device");
            let device_type: u32 = device_proxy
                .get(device_iface, "DeviceType")
                .await?
                .try_into()
                // See docs link below
                .expect("DeviceType should be u32");

            // NMDeviceType = 2 is a Wi-Fi device
            // See: https://networkmanager.dev/docs/api/latest/nm-dbus-types.html#NMDeviceType
            if device_type == 2 {
                let wireless_iface = InterfaceName::from_str_unchecked(
                    "org.freedesktop.NetworkManager.Device.Wireless",
                );
                let wireless_properties = device_proxy.get_all(wireless_iface).await?;

                if let Some(access_point_value) = wireless_properties.get("ActiveAccessPoint") {
                    let access_point_path: ObjectPath = access_point_value.downcast_ref()?;
                    let access_point_proxy =
                        PropertiesProxy::new(&connection, NM_SERVICE_NAME, access_point_path)
                            .await?;
                    let acc_point_iface = InterfaceName::from_str_unchecked(
                        "org.freedesktop.NetworkManager.AccessPoint",
                    );

                    let ssid: Option<String> = access_point_proxy
                        .get(acc_point_iface.clone(), "Ssid")
                        .await
                        .map(|s| {
                            s.downcast_ref::<Array>()
                                // See: https://networkmanager.dev/docs/api/latest/gdbus-org.freedesktop.NetworkManager.AccessPoint.html#gdbus-property-org-freedesktop-NetworkManager-AccessPoint.Ssid
                                .expect("Ssid should be list of bytes")
                                .try_into()
                                .expect("Should be able to convert Array of bytes to Vec<u8>")
                        })
                        .map(|v: Vec<u8>| String::from_utf8_lossy(&v).to_string())
                        .ok();

                    let signal: Option<Percentage> = access_point_proxy
                        .get(acc_point_iface, "Strength")
                        .await
                        .ok()
                        .and_then(|v| u8::try_from(v).ok())
                        .map(Percentage::from);

                    if let (Some(ssid), Some(signal)) = (ssid, signal) {
                        return Ok(ConnectionData::Wireless { signal, ssid });
                    }
                }
            }
        }

        Ok(ConnectionData::Wired)
    }

    /// Checks if capslock / numlock are enabled
    ///
    /// Used in [``Self::update``]
    async fn key_states() -> std::io::Result<(bool, bool)> {
        // Helper function to read the brightness of the given path
        let read_brightness = async |path: &str| {
            let content = fs::read_to_string(path).await?;
            Ok::<u32, std::io::Error>(
                content
                    .trim()
                    .parse()
                    .expect("Value of brightness file should always be a number"),
            )
        };

        let led_dir = Path::new("/sys/class/leds");
        let mut entries = fs::read_dir(led_dir).await?;

        let mut capslock_brightness_sum = 0;
        let mut numlock_brightness_sum = 0;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(file_name) = path.file_name() {
                let file_name_str = file_name.to_string_lossy();
                let brightness_path = path.join("brightness");

                // Check if the directory name matches the Caps Lock or Num Lock pattern
                if CAPSLOCK_PATTERN.is_match(&file_name_str) && brightness_path.exists() {
                    capslock_brightness_sum +=
                        read_brightness(&brightness_path.to_string_lossy()).await?;
                } else if NUMLOCK_PATTERN.is_match(&file_name_str) && brightness_path.exists() {
                    numlock_brightness_sum +=
                        read_brightness(&brightness_path.to_string_lossy()).await?;
                }
            }
        }

        Ok((capslock_brightness_sum > 0, numlock_brightness_sum > 0))
    }

    /// Get's the current volume of the default audio output
    ///
    /// If the default audio sink is muted returns `-1`
    ///
    /// Used in [``Self::update``]
    fn volume() -> alsa::Result<Percentage> {
        let mixer = Mixer::new("default", true)?;

        let selem_id = SelemId::new("Master", 0);
        let selem = mixer
            .find_selem(&selem_id)
            .expect("Default card should have Master.");

        let max = selem.get_playback_volume_range().1;
        for o in SelemChannelId::all() {
            if let Ok(volume) = selem.get_playback_volume(*o) {
                let muted = selem.get_playback_switch(*o)? == 0;

                if muted {
                    return Ok(Percentage::from(-1.0));
                }

                #[allow(clippy::cast_precision_loss)]
                return Ok((volume as f32 / max as f32).into());
            }
        }

        Ok(Percentage::default())
    }
}

/// Data component of [``SystemState``]
#[derive(Debug, Clone)]
// TODO: Impl Default on this
pub struct SystemStateData {
    /// CPU usage
    pub cpu_usage: Percentage,
    /// Amount of memory on the system (only RAM no SWAP) in bytes
    pub total_mem: u64,
    /// Amount of memory in use (only RAM no SWAP) in bytes
    pub used_mem: u64,
    /// Memory (only RAM no SWAP) usage
    pub mem_usage: Percentage,
    /// The time
    pub time: OffsetDateTime,
    /// The current workspace number
    pub workspace: i32,
    /// Data about the network connection
    pub network: ConnectionData,
    /// Battery Charge
    pub battery: Percentage,
    /// Battery Status
    pub battery_status: BatteryStatus,
    /// List of data about different disks on the system
    pub disks: Arc<[DiskData]>,
    /// If there are currently any devices connected via Bluetooth
    pub bluetooth: bool,
    /// If capslock is active
    pub capslock: bool,
    /// If numlock is active
    pub numlock: bool,
    /// Volume of the default audio output
    pub volume: Percentage,
}

/// Information about a disk
///
/// See: [``sysinfo::Disks``]
#[derive(Debug, Clone)]
pub struct DiskData {
    /// Name
    pub name: OsString,
    /// Total space (in bytes)
    pub size: u64,
    /// Free space (in bytes)
    pub free: u64,
    /// Space used
    pub used: Percentage,
}

/// Data about a network connection
#[derive(Debug, Default, Clone, PartialEq)]
pub enum ConnectionData {
    /// Connection is wired
    Wired,
    /// Connection is wireless
    Wireless {
        /// Signal strength
        signal: Percentage,
        /// SSID of the Wi-Fi network
        ssid: String,
    },
    /// There is currently no connection to the internet
    #[default]
    None,
}

/// State of a battery
#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum BatteryStatus {
    /// Loosing charge
    Discharging,
    /// Being charged
    Charging,
    /// Any other states
    ///
    /// If any other states are encountered they should be added to this enum. This is only
    /// intended to act as a fallback and for [``Default``].
    #[default]
    Unknown,
}

impl From<&str> for BatteryStatus {
    fn from(value: &str) -> Self {
        match value {
            "Charging" => BatteryStatus::Charging,
            "Discharging" => BatteryStatus::Discharging,
            _ => BatteryStatus::Unknown,
        }
    }
}
