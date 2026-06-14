//! Items relating to CSS
use std::{
    collections::HashSet,
    fmt::Display,
    ops::{Deref, DerefMut},
};

use strum::{AsRefStr, Display, EnumIs, EnumIter, EnumMessage, EnumString};

/// All CSS classes used thought the project
///
/// By having these be an enum it is easy to see which classes exist and change their names
/// through the project more easily.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    AsRefStr,
    Display,
    EnumMessage,
    EnumString,
    EnumIs,
    EnumIter,
)]
#[strum(serialize_all = "kebab-case")]
#[allow(
    missing_docs,
    reason = "The individual class names have no further meaning which would require a doc comment."
)]
pub enum Class {
    MainWindow,

    Left,
    Center,
    Right,

    Icon,
    Label,
    Active,
    Disabled,
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

    TimePlayingLabel,
    TimePlayingProgressbar,

    OskButton,

    // Launcher
    LauncherMainWindow,
    OuterBox,
    MainEntry,
    ResultsBox,

    // Osk
    OskMainWindow,
    OskMainBox,

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
    OskEscape,
    OskLayoutSwitcher,
    OskFn,
    OskArrow,
    OskDel,

    OskNormal,
    OskKeyActive,

    OskCloseButton,
    OskLockButton,
}

/// A list of [`Class`]es
///
/// Ensures there are no duplicates and provides conversions
#[derive(Debug, Clone)]
pub struct ClassList {
    /// The classes
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

/// Convenience macro to create an array of class strings
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
