use xkbcommon::xkb::{self, Keycode};

fn keymap() -> xkb::Keymap {
    xkb::Keymap::new_from_names(
        &xkb::Context::new(xkb::CONTEXT_NO_FLAGS),
        "",
        "",
        "de",
        "",
        None,
        xkb::COMPILE_NO_FLAGS,
    )
    .expect("Keymap creation should never fail.")
}

pub fn to_char(code: u32, mods: u32) -> Option<char> {
    let level = if daemon::osk::Mod::Alt.contained_in(mods) {
        2
    } else {
        u32::from(daemon::osk::Mod::Shift.contained_in(mods))
    };
    let keymap = keymap();
    let syms = keymap.key_get_syms_by_level(Keycode::new(code), 0, level);

    syms.first().and_then(|v| v.key_char())
}

pub fn to_chars(code: u32) -> [Option<char>; 3] {
    let keymap = keymap();

    let mut arr = [None; 3];

    for level in 0..=2 {
        let syms = keymap.key_get_syms_by_level(Keycode::new(code), 1, level);

        arr[level as usize] = syms.first().and_then(|v| v.key_char());
    }

    arr
}
