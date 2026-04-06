// hotkeys.rs — Global hotkey management

use global_hotkey::{hotkey::{HotKey, Modifiers, Code}, GlobalHotKeyManager};
use std::str::FromStr;

pub fn create_hotkey(mods: u32, key_code: u32, id: u32) -> Option<HotKey> {
    let mut m = Modifiers::empty();
    if mods & 1 != 0 { m |= Modifiers::SHIFT; }
    if mods & 2 != 0 { m |= Modifiers::CONTROL; }
    if mods & 4 != 0 { m |= Modifiers::ALT; }
    if mods & 8 != 0 { m |= Modifiers::SUPER; }

    let code = match key_code {
        0x1F => Code::KeyS,
        0x09 => Code::KeyC,
        0x1E => Code::KeyA,
        _ => return None,
    };

    let mut hk = HotKey::new(Some(m), code);
    hk.id = id;
    Some(hk)
}

pub struct HotKeyManager {
    manager: GlobalHotKeyManager,
}

impl HotKeyManager {
    pub fn new() -> Self {
        Self {
            manager: GlobalHotKeyManager::new().unwrap(),
        }
    }

    pub fn register(&self, hotkey: HotKey) -> Result<(), String> {
        self.manager.register(hotkey).map_err(|e| e.to_string())
    }

    pub fn unregister(&self, hotkey: HotKey) -> Result<(), String> {
        self.manager.unregister(hotkey).map_err(|e| e.to_string())
    }
}
