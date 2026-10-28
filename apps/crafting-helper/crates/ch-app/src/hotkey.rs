//! The hotkey's rules: the default, and the one combination it may never
//! be, the game's own item copy (Ctrl+Alt+C), in any spelling.
//!
//! The hotkey is registered by the glue with `tauri-plugin-global-shortcut`
//! (a registered system hotkey, not a hook). A key registered that way
//! reaches the app with the game in front (docs/RUNTIME_DATA_MODELS.md,
//! "Item search box", measured for Ctrl+Alt+W); other combinations are not
//! measured.

/// The hotkey on a first launch.
pub const DEFAULT_HOTKEY: &str = "Ctrl+Alt+X";
/// The game's own item copy, which the hotkey may never be.
pub const GAME_ITEM_COPY: &str = "Ctrl+Alt+C";

/// Whether `hotkey` is Ctrl+Alt+C in any spelling: any case, any modifier
/// order, `Control`, `CommandOrControl` or `CmdOrCtrl` for Ctrl, `Option`
/// for Alt, and `C` or `KeyC` for the key. Exactly those two modifiers: a
/// combination with Shift or Super added is another key.
pub fn is_game_item_copy(hotkey: &str) -> bool {
    let mut ctrl = 0;
    let mut alt = 0;
    let mut c = 0;
    for part in hotkey.split('+') {
        match part.trim().to_ascii_lowercase().as_str() {
            "ctrl" | "control" | "commandorcontrol" | "cmdorctrl" => ctrl += 1,
            "alt" | "option" => alt += 1,
            "c" | "keyc" => c += 1,
            _ => return false,
        }
    }
    ctrl == 1 && alt == 1 && c == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_spelling_of_the_item_copy_is_caught() {
        for h in [
            "Ctrl+Alt+C",
            "ctrl+alt+c",
            "Alt+Ctrl+C",
            "C+Alt+Ctrl",
            "Control+Alt+KeyC",
            "CommandOrControl+Alt+C",
            "CmdOrCtrl+Alt+c",
            "ALT + CONTROL + KEYC",
            "Ctrl+Option+C",
        ] {
            assert!(is_game_item_copy(h), "{h}");
        }
        // Negative controls: other keys and other modifier sets.
        for h in [
            "Ctrl+Alt+X",
            "Ctrl+C",
            "Alt+C",
            "Ctrl+Alt+Shift+C",
            "Super+Ctrl+Alt+C",
            "Ctrl+Alt+V",
            "Ctrl+Ctrl+Alt+C",
            "Ctrl+Alt+KeyX",
            "",
        ] {
            assert!(!is_game_item_copy(h), "{h}");
        }
    }
}
