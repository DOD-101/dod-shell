mod hardware;

use common::types::Percentage;
use zbus::zvariant;

/// Generates the [`System`] state struct together with its D-Bus property getters.
///
/// Each field becomes a struct field and a `#[zbus(property)]` getter of the same name.
macro_rules! system {
    (
        $(
            $(#[$field_meta:meta])*
            $field:ident: $ty:ty
        ),* $(,)?
    ) => {
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
        pub struct System {
            $(
                $(#[$field_meta])*
                pub $field: $ty,
            )*
        }

        #[zbus::interface(
            name = "dod.shell.Daemon.System",
            proxy(
                gen_blocking = false,
                default_path = "/dod/shell/Daemon",
                default_service = "dod.shell.Daemon"
            )
        )]
        #[allow(
            clippy::clone_on_copy,
            reason = "D-Bus property getters must return an owned value."
        )]
        impl System {
            $(
                $(#[$field_meta])*
                #[zbus(property)]
                fn $field(&self) -> $ty {
                    self.$field.clone()
                }
            )*
        }
    };
}

system! {
    /// CPU usage
    cpu: Percentage,
    /// Memory (only RAM no SWAP) usage
    memory: Percentage,
    /// The current workspace number
    workspace: i32,
    /// Data about the network connection
    network: ConnectionData,
    /// Data about the Battery
    battery: zvariant::Optional<BatteryData>,
    /// List of data about different disks on the system
    disks: Vec<DiskData>,
    /// If there are currently any devices connected via Bluetooth
    bluetooth: bool,
    /// If capslock is active
    capslock: bool,
    /// If numlock is active
    numlock: bool,
    /// Volume of the default audio output
    volume: Percentage,
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
    /// Losing charge
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
