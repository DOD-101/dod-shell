//! Build script to bundle icons

fn main() {
    relm4_icons_build::bundle_icons(
        // Name of the file that will be generated at `OUT_DIR`
        "icon_names.rs",
        // Optional app ID
        Some("dod-shell.bar"),
        // Custom base resource path:
        // * defaults to `/com/example/myapp` in this case if not specified explicitly
        // * or `/org/relm4` if app ID was not specified either
        None::<&str>,
        // Directory with custom icons
        Some("../../icons"),
        // List of icons to include
        [
            "processor",
            "harddisk",
            "ram-filled",
            "radiowaves-1",
            "radiowaves-2",
            "radiowaves-3",
            "radiowaves-4",
            "radiowaves-5",
            "bluetooth",
            "lan",
            "keyboard-caps-lock",
            "speaker-0-filled",
            "speaker-1-filled",
            "speaker-2-filled",
            "speaker-off-filled",
            "speaker-mute-filled",
            "battery-level-100",
            "battery-level-90",
            "battery-level-80",
            "battery-level-70",
            "battery-level-60",
            "battery-level-50",
            "battery-level-40",
            "battery-level-30",
            "battery-level-100-charged",
            "battery-level-0-charging",
            "battery-low",
            "battery-missing",
            "keyboard-filled",
        ],
    );
}
