//! Build script to bundle icons

fn main() {
    relm4_icons_build::bundle_icons(
        // Name of the file that will be generated at `OUT_DIR`
        "icon_names.rs",
        // Optional app ID
        Some("dod-shell.osk"),
        // Custom base resource path:
        // * defaults to `/com/example/myapp` in this case if not specified explicitly
        // * or `/org/relm4` if app ID was not specified either
        None::<&str>,
        // Directory with custom icons
        Some("../../icons"),
        // List of icons to include
        ["lock-small", "lock-small-open", "cross-small"],
    );
}
