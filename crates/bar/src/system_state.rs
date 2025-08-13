// TODO: Improve performance
// Baseline ~84ms
use std::{
    collections::HashMap,
    convert::TryInto,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use common::config::APP_CONFIG;

use alsa::{
    Mixer,
    mixer::{SelemChannelId, SelemId},
};
use hyprland::shared::HyprDataActive;
use regex::Regex;
use relm4::SharedState;
use sysinfo::{Disks, System};
use time::OffsetDateTime;
use zbus::{
    Proxy,
    fdo::PropertiesProxy,
    names::InterfaceName,
    zvariant::{Array, ObjectPath, OwnedObjectPath, OwnedValue, Value},
};

const NM_SERVICE_NAME: &str = "org.freedesktop.NetworkManager";

pub static SYSTEM_STATE: SharedState<SystemState> = SharedState::new();

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
                network: ConnectionData::default(),
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

        // NOTE: Not sure if there is a better way
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(state.update());

        state
    }
}

impl SystemState {
    #[allow(clippy::cast_precision_loss)]
    async fn update(&mut self) {
        self.sys.refresh_all();
        self.data.cpu_usage = self.sys.global_cpu_usage();
        self.data.used_mem = self.sys.used_memory();
        self.data.mem_usage = self.data.used_mem as f32 / self.data.total_mem as f32;
        self.data.time = OffsetDateTime::now_local().expect("Failed to get time offset.");
        self.data.workspace = hyprland::data::Workspace::get_active().unwrap().id;

        self.disks.refresh(true);

        // TODO: There should be a way for the user to know which disks are available
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

        let (bluetooth, network) = tokio::join!(self.bluetooth(), self.network());

        self.data.bluetooth = bluetooth
            .inspect_err(|e| log::error!("Failed to update bluetooth information: {e}"))
            .unwrap_or_default();

        self.data.network = network
            .inspect_err(|e| log::error!("Failed to update network information: {e}"))
            .unwrap_or_default();

        (self.data.capslock, self.data.numlock) = get_key_states().unwrap_or_default();

        self.data.volume = SystemState::volume()
            .inspect_err(|e| log::error!("Failed to update volume information: {e}"))
            .unwrap_or_default();
    }

    pub fn get_data(&self) -> &SystemStateData {
        &self.data
    }

    /// Checks if any devices are connected via bluetooth
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

    /// Gathers information about the current internet connection
    ///
    /// See: [`ConnectionData`]
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

                    let signal: Option<u8> = access_point_proxy
                        .get(acc_point_iface, "Strength")
                        .await
                        .ok()
                        .and_then(|v| v.try_into().ok());

                    if let (Some(ssid), Some(signal)) = (ssid, signal) {
                        return Ok(ConnectionData::Wireless { signal, ssid });
                    }
                }
            }
        }

        Ok(ConnectionData::Wired)
    }

    /// Get's the current volume of the default audio output
    ///
    /// If the default audio sink is muted returns `-1`
    fn volume() -> alsa::Result<f64> {
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
                    return Ok(-1.0);
                }

                #[allow(clippy::cast_precision_loss)]
                return Ok(volume as f64 / max as f64);
            }
        }

        Ok(0.0)
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
    pub network: ConnectionData,
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
    pub volume: f64,
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

/// Data about the current internet connection
#[derive(Debug, Default, Clone, PartialEq)]
pub enum ConnectionData {
    /// The connection to the internet is wired
    Wired,
    /// The connection to the internet is wireless
    Wireless {
        /// The signal strength as a percentage
        signal: u8,
        /// The SSID of the current Wi-Fi network connected to
        ssid: String,
    },
    /// There is currently no connection to the internet
    #[default]
    None,
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
