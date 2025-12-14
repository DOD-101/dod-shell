use std::{
    collections::HashSet,
    fmt::Display,
    ops::{Deref, DerefMut},
};

use strum::{AsRefStr, Display, EnumIs, EnumMessage, EnumString};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, AsRefStr, Display, EnumMessage, EnumString, EnumIs,
)]
#[strum(serialize_all = "kebab-case")]
pub enum Class {
    MainWindow,

    Left,
    Center,
    Right,

    Icon,
    Label,
    Active,
    Muted,

    // Bar
    BarMainWindow,
    BarCenterbox,

    Battery,
    BatteryLow,
    HardwareInfo,
    Cpu,
    Ram,
    InternetNameRevealer,

    InternetIcon,
    BluetoothIcon,
    CapsLockIcon,
    NumLockIcon,

    Workspaces,
    WorkspacesInner,
    Workspace,
    WorkspaceButton,

    LabelIcon,
    LabelIconLabel,
    LabelIconIcon,

    OskButton,

    // Launcher
    LauncherMainWindow,
    OuterBox,
    MainEntry,
    ResultsBox,

    // Osk
    OskMainWindow,

    OskRow,

    OskKey,
    OskMod,
    OskCtrl,
    OskAlt,
    OskAltGr,
    OskSuper,
    OskShift,
    OskShiftLock,

    OskUtf,
    OskCode,
    OskEnter,
    OskBackspace,
    OskSpace,
    OskSpacer,
    OskEscape,

    OskNormal,
    OskKeyActive,
}

#[derive(Debug, Clone)]
pub struct ClassList {
    classes: HashSet<Class>,
}

impl Display for ClassList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ",
            self.classes.iter().fold(
                String::with_capacity(self.classes.len() * 4),
                |mut acc, v| {
                    acc.push_str(v.as_ref());

                    acc
                }
            )
        )
    }
}

impl Deref for ClassList {
    type Target = HashSet<Class>;
    fn deref(&self) -> &Self::Target {
        &self.classes
    }
}

impl DerefMut for ClassList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.classes
    }
}

impl<'a> From<&'a ClassList> for Vec<&'a str> {
    fn from(value: &'a ClassList) -> Self {
        value.classes.iter().map(AsRef::as_ref).collect()
    }
}

impl From<Vec<Class>> for ClassList {
    fn from(value: Vec<Class>) -> Self {
        Self {
            classes: value.into_iter().collect(),
        }
    }
}

impl From<HashSet<Class>> for ClassList {
    fn from(value: HashSet<Class>) -> Self {
        Self { classes: value }
    }
}

impl<const N: usize> From<&[Class; N]> for ClassList {
    fn from(value: &[Class; N]) -> Self {
        Self {
            classes: value.iter().copied().collect(),
        }
    }
}

/// Convenience macro to allow for creating an array of class strings
#[macro_export]
macro_rules! classes {
    ( $($class:ident),+ $(,)? ) => {
        [ $($crate::css::Class::$class.as_ref(), )* ]

    };
}

#[cfg(test)]
mod test {
    #[test]
    fn classes_macro() {
        let a = classes!(Label, BatteryLow);

        assert_eq!(a, ["label", "battery-low"]);
    }
}
