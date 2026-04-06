// win_utils.rs — Windows Registry and Shell integration utilities

use std::path::Path;
use std::ptr;
use windows::core::PCWSTR;
use windows::Win32::System::Registry::{
    RegCreateKeyExW, RegDeleteTreeW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_ALL_ACCESS,
    REG_OPTION_NON_VOLATILE, REG_SZ,
};
use windows::Win32::UI::Shell::{SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST};

pub const CLSID_STR: &str = "{D8E26C78-5B7E-4E38-9B7E-4E389B7E4E38}";
pub const PROG_ID: &str = "UriViewer.AssocFile";

pub const IMAGE_EXTENSIONS: &[&str] = &[
    "ANI", "BMP", "CAL", "EMF", "FAX", "GIF", "HDP", "ICO", "JPE", "JPEG",
    "JPG", "MAC", "PBM", "PCD", "PCX", "PGM", "PNG", "PPM", "PSD", "RAS",
    "TGA", "TIF", "TIFF", "WMF",
];

pub const CAD_EXTENSIONS: &[&str] = &[
    "CGM", "DWG", "DWF", "DXF", "IGES", "OBJ", "PLT", "STEP", "STL", "SVG", "3DS",
];

pub fn get_all_extensions() -> Vec<String> {
    let mut ext = IMAGE_EXTENSIONS.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    ext.extend(CAD_EXTENSIONS.iter().map(|s| s.to_string()));
    ext
}

pub fn register_shell_extension(dll_path: &Path) -> Result<(), String> {
    let clsid_key = format!("Software\\Classes\\CLSID\\{}", CLSID_STR);

    // 1. Register CLSID
    set_reg_value(&clsid_key, "", "UriViewer Shell Extension")?;
    set_reg_value(
        &format!("{}\\InprocServer32", clsid_key),
        "",
        &dll_path.to_string_lossy(),
    )?;
    set_reg_value(
        &format!("{}\\InprocServer32", clsid_key),
        "ThreadingModel",
        "Apartment",
    )?;

    // 2. Register Context Menu Handlers
    let handler_paths = [
        "Software\\Classes\\*\\shellex\\ContextMenuHandlers\\UriViewerExt",
        "Software\\Classes\\SystemFileAssociations\\image\\shellex\\ContextMenuHandlers\\UriViewerExt",
    ];

    for path in handler_paths {
        set_reg_value(path, "", CLSID_STR)?;
    }

    notify_shell();
    Ok(())
}

pub fn unregister_shell_extension() {
    let clsid_key = format!("Software\\Classes\\CLSID\\{}", CLSID_STR);
    let handler_paths = [
        "Software\\Classes\\*\\shellex\\ContextMenuHandlers\\UriViewerExt",
        "Software\\Classes\\SystemFileAssociations\\image\\shellex\\ContextMenuHandlers\\UriViewerExt",
    ];

    for path in handler_paths {
        let _ = delete_reg_key(path);
    }
    let _ = delete_reg_key(&clsid_key);
    
    // Cleanup old versions
    let _ = delete_reg_key("Software\\Classes\\CLSID\\{C8E26C78-5B7E-4E38-9B7E-4E389B7E4E38}");
    
    notify_shell();
}

pub fn register_file_association(ext: &str) -> Result<(), String> {
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_str = exe_path.to_string_lossy();

    // 1. Create ProgID
    let prog_key = format!("Software\\Classes\\{}", PROG_ID);
    set_reg_value(&prog_key, "", "UriViewer Image File")?;
    set_reg_value(&format!("{}\\DefaultIcon", prog_key), "", &format!("{},0", exe_str))?;
    set_reg_value(&format!("{}\\shell\\open\\command", prog_key), "", &format!("\"{}\" \"%1\"", exe_str))?;

    // 2. Associate Extension
    let ext_lower = ext.to_lowercase();
    let ext_key = format!("Software\\Classes\\.{}", ext_lower);
    set_reg_value(&ext_key, "", PROG_ID)?;
    
    // Windows Explorer often uses OpenWithProgids too
    let open_with_key = format!("{}\\OpenWithProgids", ext_key);
    set_reg_value(&open_with_key, PROG_ID, "")?;

    notify_shell();
    Ok(())
}

pub fn unregister_file_association(ext: &str) {
    let ext_lower = ext.to_lowercase();
    let ext_key = format!("Software\\Classes\\.{}", ext_lower);
    
    // We only remove our ProgID link. 
    // If the default was PROG_ID, we might want to clear it, but Windows handled it usually.
    // To be safe, we just remove our ProgID from OpenWithProgids
    let _ = delete_reg_value(&format!("{}\\OpenWithProgids", ext_key), PROG_ID);
    
    // If the main association is ours, we might want to let it be or clear it.
    // Many apps leave it. Let's clear if it's ours.
    if let Ok(val) = get_reg_value(&ext_key, "") {
        if val == PROG_ID {
            let _ = set_reg_value(&ext_key, "", ""); 
        }
    }

    notify_shell();
}

pub fn notify_shell() {
    unsafe {
        SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None);
    }
}

// ── Registry Helpers ──

pub fn set_reg_value(key_path: &str, value_name: &str, value_data: &str) -> Result<(), String> {
    unsafe {
        let wide_key: Vec<u16> = key_path.encode_utf16().chain(std::iter::once(0)).collect();
        let mut hkey = HKEY::default();

        let status = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(wide_key.as_ptr()),
            0,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_ALL_ACCESS,
            None,
            &mut hkey,
            None,
        );

        if status.is_err() {
            return Err(format!("Failed to create registry key {}: {:?}", key_path, status));
        }

        let wide_name: Vec<u16> = value_name.encode_utf16().chain(std::iter::once(0)).collect();
        let wide_data: Vec<u16> = value_data.encode_utf16().chain(std::iter::once(0)).collect();

        let status = RegSetValueExW(
            hkey,
            PCWSTR(if value_name.is_empty() { ptr::null() } else { wide_name.as_ptr() }),
            0,
            REG_SZ,
            Some(std::slice::from_raw_parts(wide_data.as_ptr() as *const u8, wide_data.len() * 2)),
        );

        if status.is_err() {
            return Err(format!("Failed to set registry value {}: {:?}", value_name, status));
        }
    }
    Ok(())
}

pub fn delete_reg_key(key_path: &str) -> Result<(), String> {
    unsafe {
        let wide_key: Vec<u16> = key_path.encode_utf16().chain(std::iter::once(0)).collect();
        let _ = RegDeleteTreeW(HKEY_CURRENT_USER, PCWSTR(wide_key.as_ptr()));
    }
    Ok(())
}

pub fn delete_reg_value(key_path: &str, value_name: &str) -> Result<(), String> {
    use windows::Win32::System::Registry::{RegOpenKeyExW, RegDeleteValueW};
    unsafe {
        let wide_key: Vec<u16> = key_path.encode_utf16().chain(std::iter::once(0)).collect();
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(wide_key.as_ptr()), 0, KEY_ALL_ACCESS, &mut hkey).is_ok() {
            let wide_name: Vec<u16> = value_name.encode_utf16().chain(std::iter::once(0)).collect();
            let _ = RegDeleteValueW(hkey, PCWSTR(wide_name.as_ptr()));
        }
    }
    Ok(())
}

pub fn get_reg_value(key_path: &str, value_name: &str) -> Result<String, String> {
    use windows::Win32::System::Registry::{RegOpenKeyExW, RegQueryValueExW, KEY_READ};
    unsafe {
        let wide_key: Vec<u16> = key_path.encode_utf16().chain(std::iter::once(0)).collect();
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(wide_key.as_ptr()), 0, KEY_READ, &mut hkey).is_err() {
            return Err("Key not found".into());
        }

        let wide_name: Vec<u16> = value_name.encode_utf16().chain(std::iter::once(0)).collect();
        let mut data_len = 0u32;
        let p_name = if value_name.is_empty() { ptr::null() } else { wide_name.as_ptr() };
        
        let _ = RegQueryValueExW(hkey, PCWSTR(p_name), None, None, None, Some(&mut data_len));
        if data_len == 0 { return Err("Value not found".into()); }

        let mut data = vec![0u8; data_len as usize];
        if RegQueryValueExW(hkey, PCWSTR(p_name), None, None, Some(data.as_mut_ptr()), Some(&mut data_len)).is_ok() {
            let wide_data = std::slice::from_raw_parts(data.as_ptr() as *const u16, data.len() / 2);
            let mut s = String::from_utf16_lossy(wide_data);
            if let Some(pos) = s.find('\0') { s.truncate(pos); }
            Ok(s)
        } else {
            Err("Failed to query".into())
        }
    }
}
