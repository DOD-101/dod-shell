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

pub fn to_chars(code: u32) -> [Option<char>; 3] {
    let keymap = keymap();

    let mut arr = [None; 3];

    for level in 0..=2 {
        let syms = keymap.key_get_syms_by_level(Keycode::new(code), 1, level);

        arr[level as usize] = syms.first().and_then(|v| v.key_char());
    }

    arr
}
