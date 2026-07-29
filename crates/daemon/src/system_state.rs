//! This module contains items relating to getting "State" (aka. Information)
//! about the system and other processes running on the system
//!
//! It can be thought of as an interface between information gathered from outside the shell and
//! the components which need that information.
//!
//! The main type is [``SystemState``]
use std::{
    collections::HashMap,
    convert::TryInto,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use anyhow::Result;
use hyprland::shared::HyprDataActive;
use regex::Regex;
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};
use tokio::fs;
use zbus::{
    Proxy,
    fdo::PropertiesProxy,
    interface,
    names::InterfaceName,
    zvariant::{self, Array, ObjectPath, OwnedObjectPath, OwnedValue, Value},
};

use common::types::Percentage;

/// Dbus service name for `NetworkManager` used by [``SystemState::update_network``]
const NM_SERVICE_NAME: &str = "org.freedesktop.NetworkManager";

/// [``Regex``] used by [``SystemState::update_key_states``]
static CAPSLOCK_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"input\d+::capslock").unwrap());
/// [``Regex``] used by [``SystemState::update_key_states``]
static NUMLOCK_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"input\d+::numlock").unwrap());

/// All of the State (aka. Information) gathered from the system
///
/// Provides the [``Self::update``] method for updating said state.
///
/// Internally holds [``SystemStateData``] to actually hold all of the data.
///
/// Other than the data itself it contains Objects needed to update parts of the state, which
/// shouldn't be re-created each time [``Self::update``] is run due to performance reasons
///
/// # Dbus
///
/// This struct implements [``zbus::object_server::Interface``], which means it acts as a dbus
/// interface. For available zbus methods and properties see [``SystemStateProxy``]
#[derive(Debug, Default)]
pub struct SystemState {
    /// Used in [``Self::update``]
    sys: System,
    /// Used in [``Self::update``]
    disks: Disks,
    /// Actual data
    data: SystemStateData,
    /// The current config
    config: common::Config,
    /// Used in [`Self::update_volume`]
    pulse_volume: pulse_ref::PulseRef,
}

impl SystemState {
    /// Set the internal [``common::Config``]
    ///
    /// This is used primarily if there has been a change to the on-disk config file.
    ///
    /// For more information on the updating process see [``crate::config``]
    pub fn set_config(&mut self, config: common::Config) {
        self.config = config;
    }

    /// Used for updating the state
    ///
    /// ## Performance
    ///
    /// Currently this method takes 6-10ms to run.
    /// It is critical that care is taken to maintain optimal performance, since this function
    /// hanging will cause large parts of the shell to hang themselves or break.
    #[allow(
        clippy::cast_precision_loss,
        reason = "Precision loss only occurs when calculating percentages, where we don't care since they are just for display."
    )]
    #[allow(clippy::missing_panics_doc, reason = "See expect msg.")]
    pub async fn update(&mut self) {
        let connection = zbus::Connection::system().await.expect("Shouldn't fail to connect to system dbus, since we have interacted with dbus before this point already.");

        self.sys.refresh_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
                .with_memory(MemoryRefreshKind::nothing().with_ram()),
        );

        self.data.cpu_usage = (f64::from(self.sys.global_cpu_usage()) / 100.0).into();
        self.data.used_mem = self.sys.used_memory();
        self.data.total_mem = self.sys.total_memory();
        self.data.mem_usage = (self.data.used_mem as f64 / self.data.total_mem as f64).into();
        self.data.workspace = hyprland::data::Workspace::get_active()
            .inspect_err(|e| log::error!("Failed to get an active workspace: {e}"))
            .map_or(0, |w| w.id);

        self.disks.refresh(true);

        self.data.disks = self
            .disks
            .list()
            .iter()
            .map(|d| {
                let size = d.total_space();
                let free = d.available_space();
                let used = ((size as f64 - free as f64) / size as f64).into();
                DiskData {
                    name: d.name().to_string_lossy().to_string(),
                    size,
                    free,
                    used,
                }
            })
            .collect();

        let (bluetooth, network, key_states, battery_data) = tokio::join!(
            self.update_bluetooth(&connection),
            self.update_network(&connection),
            Self::update_key_states(),
            self.update_battery()
        );

        self.data.bluetooth = bluetooth
            .inspect_err(|e| log::error!("Failed to update bluetooth information: {e}"))
            .unwrap_or_default();

        self.data.network = network
            .inspect_err(|e| log::error!("Failed to update network information: {e}"))
            .unwrap_or_default();

        (self.data.capslock, self.data.numlock) = key_states
            .inspect_err(|e| log::error!("Failed to update key state information: {e}"))
            .unwrap_or_default();

        match battery_data {
            Some(Ok(data)) => self.data.battery = zvariant::Optional::from(Some(data)),
            Some(Err(e)) => log::error!("Failed to update battery information: {e}"),
            None => (),
        }

        self.data.volume = self.pulse_volume.get_volume();
    }

    /// Checks if any devices are connected via bluetooth
    ///
    /// Used in [``Self::update``]
    async fn update_bluetooth(&self, connection: &zbus::Connection) -> zbus::Result<bool> {
        // Create a proxy to interact with BlueZ's ObjectManager interface
        // ObjectManager provides a way to discover all available objects and their interfaces
        // BlueZ: https://git.kernel.org/pub/scm/bluetooth/bluez.git/tree/
        let bluez_proxy = zbus::Proxy::new(
            connection,
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
                if let Some(connected_value) = device_props.get("Connected")
                    && bool::try_from(connected_value).is_ok_and(|v| v)
                {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Gets information about the battery, if one is set in the shell [Config](``common::config::Config``)
    ///
    /// Used in [``Self::update``]
    async fn update_battery(&self) -> Option<Result<BatteryData>> {
        if let Some(bat) = &self.config.bar.battery {
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
                (Ok(p), Ok(s)) => Some(Ok(BatteryData {
                    charge: p.into(),
                    status: s,
                })),
                (Err(e), _) | (_, Err(e)) => Some(Err(e.into())),
            };
        }

        None
    }

    /// Gathers information about the current internet connection
    ///
    /// Used in [``Self::update``]
    async fn update_network(&self, connection: &zbus::Connection) -> Result<ConnectionData> {
        let nm_iface = InterfaceName::from_str_unchecked(NM_SERVICE_NAME);
        let nm_proxy = Proxy::new(
            connection,
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
            let device_proxy = PropertiesProxy::new(connection, NM_SERVICE_NAME, &d).await?;

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
                let active_access_point = device_proxy
                    .get(wireless_iface, "ActiveAccessPoint")
                    .await?;

                let access_point_path: ObjectPath = active_access_point.downcast_ref()?;
                let access_point_proxy =
                    PropertiesProxy::new(connection, NM_SERVICE_NAME, access_point_path).await?;
                let acc_point_iface =
                    InterfaceName::from_str_unchecked("org.freedesktop.NetworkManager.AccessPoint");

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

        Ok(ConnectionData::Wired)
    }

    /// Checks if capslock / numlock are enabled
    ///
    /// Used in [``Self::update``]
    ///
    /// Field 0 signals if capslock is enabled
    /// Field 1 signals if numlock is enabled
    async fn update_key_states() -> Result<(bool, bool)> {
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
}

#[interface(
    name = "dod.shell.Daemon.SystemState",
    proxy(
        gen_blocking = false,
        default_path = "/dod/shell/Daemon",
        default_service = "dod.shell.Daemon"
    )
)]
impl SystemState {
    /// Dbus property to get the current data
    #[zbus(property)]
    fn state_data(&self) -> SystemStateData {
        self.data.clone()
    }
}

/// Data component of [``SystemState``]
#[derive(Debug, Clone, zvariant::Value, zvariant::OwnedValue, zvariant::Type, Default)]
pub struct SystemStateData {
    /// CPU usage
    pub cpu_usage: Percentage,
    /// Amount of memory on the system (only RAM no SWAP) in bytes
    pub total_mem: u64,
    /// Amount of memory in use (only RAM no SWAP) in bytes
    pub used_mem: u64,
    /// Memory (only RAM no SWAP) usage
    pub mem_usage: Percentage,
    /// The current workspace number
    pub workspace: i32,
    /// Data about the network connection
    pub network: ConnectionData,
    /// Data about the Battery
    pub battery: zvariant::Optional<BatteryData>,
    /// List of data about different disks on the system
    pub disks: Vec<DiskData>,
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
#[derive(Debug, Clone, zbus::zvariant::Value, zbus::zvariant::OwnedValue, zvariant::Type)]
pub struct DiskData {
    /// Name
    pub name: String,
    /// Total space (in bytes)
    pub size: u64,
    /// Free space (in bytes)
    pub free: u64,
    /// Space used
    pub used: Percentage,
}

/// Data about a network connection
#[derive(Debug, Default, Clone, PartialEq, zvariant::Type)]
#[zvariant(signature = "s")]
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

impl TryFrom<zvariant::Value<'_>> for ConnectionData {
    type Error = zvariant::Error;
    fn try_from(value: zvariant::Value<'_>) -> zvariant::Result<Self> {
        if let zvariant::Value::Structure(v) = value {
            let mut field_iter = v.into_fields().into_iter();

            return match field_iter.next() {
                Some(zvariant::Value::I32(0)) => Ok(Self::Wired),
                Some(zvariant::Value::I32(1)) => Ok(Self::Wireless {
                    signal: field_iter
                        .next()
                        .ok_or(Self::Error::IncorrectType)?
                        .try_to_owned()?
                        .try_into()?,
                    ssid: field_iter
                        .next()
                        .ok_or(Self::Error::IncorrectType)?
                        .try_into()?,
                }),
                Some(zvariant::Value::I32(2)) => Ok(Self::None),
                _ => Err(Self::Error::IncorrectType),
            };
        }

        Err(Self::Error::IncorrectType)
    }
}

impl From<ConnectionData> for zvariant::OwnedValue {
    fn from(value: ConnectionData) -> Self {
        std::convert::Into::<zvariant::Value>::into(value)
            .try_to_owned()
            .expect("Should never fail since we don't have a fd (see docs for .try_to_owned()).")
    }
}

impl TryFrom<zvariant::OwnedValue> for ConnectionData {
    type Error = zvariant::Error;

    fn try_from(value: zvariant::OwnedValue) -> zvariant::Result<Self> {
        Self::try_from(zvariant::Value::from(value))
    }
}

impl From<ConnectionData> for zvariant::Structure<'_> {
    fn from(value: ConnectionData) -> Self {
        match value {
            ConnectionData::Wired => (0, Percentage::default(), String::default()),
            ConnectionData::Wireless { signal, ssid } => (1, signal, ssid),
            ConnectionData::None => (2, Percentage::default(), String::default()),
        }
        .into()
    }
}

/// Data relating to a battery
#[derive(
    Default,
    Debug,
    Clone,
    PartialEq,
    PartialOrd,
    zbus::zvariant::Value,
    zbus::zvariant::OwnedValue,
    zvariant::Type,
)]
pub struct BatteryData {
    /// What percentage the battery is charged to
    pub charge: Percentage,
    /// The current status of the batter
    pub status: BatteryStatus,
}

/// What the battery is currently doing
#[derive(
    Default,
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    zbus::zvariant::Value,
    zbus::zvariant::OwnedValue,
    zvariant::Type,
)]
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
            "Charging" => Self::Charging,
            "Discharging" => Self::Discharging,
            _ => Self::Unknown,
        }
    }
}

mod pulse_ref {
    //! See [`PulseRef`]
    use libpulse_binding::{
        callbacks::ListResult,
        context::{Context, FlagSet, State, introspect::SinkInfo},
        mainloop::threaded::Mainloop,
        volume::Volume,
    };
    use std::{
        sync::mpsc,
        thread::{JoinHandle, spawn},
        time::Duration,
    };

    /// Lazily-initialized `PulseAudio` volume querier.
    ///
    /// Spawns a background thread that owns the `!Send` [`Mainloop`] and provides volume
    /// queries via a channel interface.
    #[derive(Debug)]
    pub struct PulseRef {
        /// Used to send messages to background thread
        sender: mpsc::Sender<Command>,
        /// Used to stop background thread on [`Drop`]
        join_handle: Option<JoinHandle<()>>,
    }

    /// Commands sent to the background thread
    enum Command {
        /// Get the current volume
        ///
        /// See: [`PulseRef::get_volume`]
        GetVolume {
            /// Used to send back response
            respond_to: mpsc::Sender<Option<(u32, bool)>>,
        },
        /// Used to stop the background thread before dropping
        Shutdown,
    }

    impl Default for PulseRef {
        fn default() -> Self {
            let (tx, rx) = mpsc::channel::<Command>();

            let handle = spawn(move || {
                let Some(mut mainloop) = Mainloop::new() else {
                    log::error!("Failed to create PulseAudio mainloop");
                    return;
                };
                let Some(mut context) = Context::new(&mainloop, "dod-shell-daemon") else {
                    log::error!("Failed to create PulseAudio context");
                    return;
                };
                if let Err(e) = context.connect(None, FlagSet::NOFLAGS, None) {
                    log::error!("Failed to connect to PulseAudio: {e:?}");
                    return;
                }
                if let Err(e) = mainloop.start() {
                    log::error!("Failed to start PulseAudio mainloop: {e:?}");
                    return;
                }

                let mut ready = false;
                for _ in 0..200 {
                    match context.get_state() {
                        State::Ready => {
                            ready = true;
                            break;
                        }
                        State::Failed | State::Terminated => {
                            log::error!("PulseAudio connection failed");
                            return;
                        }
                        _ => std::thread::sleep(Duration::from_millis(10)),
                    }
                }
                if !ready {
                    log::error!("Timed out waiting for PulseAudio connection");
                    return;
                }

                for cmd in rx {
                    match cmd {
                        Command::GetVolume { respond_to } => {
                            let result = query_volume(&context);
                            let _ = respond_to.send(result);
                        }
                        Command::Shutdown => break,
                    }
                }
            });

            Self {
                sender: tx,
                join_handle: Some(handle),
            }
        }
    }

    impl PulseRef {
        /// Get the current volume
        ///
        /// If the volume is muted the [`Percentage`] has value -1.0.
        /// If no information could be gathered it is 0 and an error is logged.
        pub fn get_volume(&self) -> common::types::Percentage {
            let (tx, rx) = mpsc::channel();
            if self
                .sender
                .send(Command::GetVolume { respond_to: tx })
                .is_err()
            {
                log::error!("PulseAudio thread died");
                return common::types::Percentage::default();
            }
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(Some((_raw_vol, true))) => common::types::Percentage::from(-1.0),
                Ok(Some((raw_vol, false))) => {
                    let ratio = f64::from(raw_vol) / f64::from(Volume::NORMAL.0);
                    common::types::Percentage::from(ratio)
                }
                Ok(None) => {
                    log::error!("PulseAudio returned no sink info");
                    common::types::Percentage::default()
                }
                Err(_) => {
                    log::error!("PulseAudio volume query timed out");
                    common::types::Percentage::default()
                }
            }
        }
    }

    /// Helper function to get the volume from the given [`Context`]
    ///
    /// Returns the volume and mute status.
    fn query_volume(context: &Context) -> Option<(u32, bool)> {
        let (tx, rx) = mpsc::channel();
        let introspector = context.introspect();
        introspector.get_sink_info_by_name(
            "@DEFAULT_SINK@",
            move |res: ListResult<&SinkInfo<'_>>| {
                if let ListResult::Item(info) = res {
                    let _ = tx.send((info.volume.avg().0, info.mute));
                }
            },
        );
        rx.recv_timeout(Duration::from_secs(1)).ok()
    }

    impl Drop for PulseRef {
        fn drop(&mut self) {
            let _ = self.sender.send(Command::Shutdown);
            if let Some(handle) = self.join_handle.take() {
                let _ = handle.join();
            }
        }
    }
}
