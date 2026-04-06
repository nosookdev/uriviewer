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
        0x41 => Code::KeyA, 0x42 => Code::KeyB, 0x43 => Code::KeyC, 0x44 => Code::KeyD,
        0x45 => Code::KeyE, 0x46 => Code::KeyF, 0x47 => Code::KeyG, 0x48 => Code::KeyH,
        0x49 => Code::KeyI, 0x4A => Code::KeyJ, 0x4B => Code::KeyK, 0x4C => Code::KeyL,
        0x4D => Code::KeyM, 0x4E => Code::KeyN, 0x4F => Code::KeyO, 0x50 => Code::KeyP,
        0x51 => Code::KeyQ, 0x52 => Code::KeyR, 0x53 => Code::KeyS, 0x54 => Code::KeyT,
        0x55 => Code::KeyU, 0x56 => Code::KeyV, 0x57 => Code::KeyW, 0x58 => Code::KeyX,
        0x59 => Code::KeyY, 0x5A => Code::KeyZ,
        
        0x30 => Code::Digit0, 0x31 => Code::Digit1, 0x32 => Code::Digit2, 0x33 => Code::Digit3,
        0x34 => Code::Digit4, 0x35 => Code::Digit5, 0x36 => Code::Digit6, 0x37 => Code::Digit7,
        0x38 => Code::Digit8, 0x39 => Code::Digit9,

        0x70 => Code::F1, 0x71 => Code::F2, 0x72 => Code::F3, 0x73 => Code::F4,
        0x74 => Code::F5, 0x75 => Code::F6, 0x76 => Code::F7, 0x77 => Code::F8,
        0x78 => Code::F9, 0x79 => Code::F10, 0x7A => Code::F11, 0x7B => Code::F12,

        // Legacy/Custom mappings
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
