//! Functions relating to [`xkb`] and working with keymaps
use xkbcommon::xkb::{self, Keycode};

/// Returns the keymap used by the osk
///
/// Currently this just statically returns the German keyboard layout, but this might change in the
/// future
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

/// Takes a key-code and returns the characters associated with it with different modifiers
///
/// Returns the characters in order of `[no-modifiers, shift, alt]`
pub fn to_chars(code: u32) -> [Option<char>; 3] {
    let keymap = keymap();

    let mut arr = [None; 3];

    for level in 0..=2 {
        let syms = keymap.key_get_syms_by_level(Keycode::new(code), 1, level);

        arr[level as usize] = syms.first().and_then(|v| v.key_char());
    }

    arr
}

/// Attempts to get the key code for a key name
///
/// Current implementation only checks level 0.
pub fn get_key_code(key_name: &str) -> Option<u32> {
    let keymap = keymap();

    let mut ret = None;

    keymap.key_for_each(|map, code| {
        if ret.is_some() {
            return;
        }

        let val = map.key_get_syms_by_level(code, 1, 0);
        if !val.is_empty() {
            let name = val[0].name();

            if name.is_some_and(|v| v == key_name) {
                ret = Some(code.raw());
            }
        }
    });

    ret
}
