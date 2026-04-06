// RustView — Lightweight multi-format image viewer
// Entry point: parse CLI args, launch eframe window

#![windows_subsystem = "windows"]

use uriviewer_ext::app::RustViewApp;

use std::path::PathBuf;
use std::ptr;
use windows::core::PCWSTR;
use windows::Win32::System::Registry::{
    RegCreateKeyExW, RegDeleteTreeW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_ALL_ACCESS,
    REG_OPTION_NON_VOLATILE, REG_SZ,
};
use windows::Win32::UI::Shell::{SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST};

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // ── Handle Automatic Registration ──
    // 개발 버전에서는 매번 컨텍스트 메뉴를 삭제하고 다시 등록하여 변경사항을 즉시 반영합니다.
    let _ = check_and_register_if_needed();

    // ── Handle Arguments ──
    let mut initial_path = None;
    let mut _convert_mode = false;

    for arg in args.iter().skip(1) {
        if arg == "--convert" {
            _convert_mode = true;
        } else if !arg.starts_with("--") {
            let p = PathBuf::from(arg);
            if p.exists() {
                initial_path = Some(p);
            }
        }
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([640.0, 480.0])
            .with_title("uriviewer")
            .with_drag_and_drop(true)
            .with_decorations(false)
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "uriviewer",
        native_options,
        Box::new(move |cc| Ok(Box::new(RustViewApp::new(cc, initial_path)))),
    )
}

fn check_and_register_if_needed() -> Result<(), String> {
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let dll_path = exe_path.with_file_name("uriviewer_ext.dll");

    // DLL이 없으면 등록 불가
    if !dll_path.exists() {
        return Ok(());
    }

    let old_clsid = "{C8E26C78-5B7E-4E38-9B7E-4E389B7E4E38}";
    let new_clsid = "{D8E26C78-5B7E-4E38-9B7E-4E389B7E4E38}";

    // 개발 중 매번 재등록
    // println!("Cleaning up old context menu entries...");
    unregister_shell_extension(old_clsid);
    unregister_shell_extension(new_clsid);
    
    // println!("Registering UriViewer extension ({:?})...", dll_path);
    register_shell_extension(new_clsid, &dll_path)?;

    unsafe {
        SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None);
    }
    
    Ok(())
}

fn unregister_shell_extension(clsid_str: &str) {
    let clsid_key = format!("Software\\Classes\\CLSID\\{}", clsid_str);
    let handler_names = ["RustView", "UriViewerExt"];
    let base_paths = [
        "Software\\Classes\\*\\shellex\\ContextMenuHandlers",
        "Software\\Classes\\SystemFileAssociations\\image\\shellex\\ContextMenuHandlers",
    ];

    for base in base_paths {
        for name in handler_names {
            let full_path = format!("{}\\{}", base, name);
            let _ = delete_reg_key(&full_path);
        }
    }

    // 구버전 정적 메뉴 항목 제거 (URI_VIEWER_OPEN, URI_VIEWER_CONVERT)
    let legacy_paths = [
        "Software\\Classes\\Applications\\rustview.exe\\shell\\URI_VIEWER_OPEN",
        "Software\\Classes\\Applications\\rustview.exe\\shell\\URI_VIEWER_CONVERT",
        "Software\\Classes\\*\\shell\\URI_VIEWER_OPEN",
        "Software\\Classes\\*\\shell\\URI_VIEWER_CONVERT",
        "Software\\Classes\\SystemFileAssociations\\image\\shell\\URI_VIEWER_OPEN",
        "Software\\Classes\\SystemFileAssociations\\image\\shell\\URI_VIEWER_CONVERT",
    ];

    for path in legacy_paths {
        let _ = delete_reg_key(path);
    }

    let _ = delete_reg_key(&clsid_key);
}

fn register_shell_extension(clsid_str: &str, dll_path: &std::path::Path) -> Result<(), String> {
    // 등록 전 항상 기존 항목(구버전 포함) 제거
    unregister_shell_extension(clsid_str);
    
    let clsid_key = format!("Software\\Classes\\CLSID\\{}", clsid_str);

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

    // 2. Register for all files AND specifically for images (UriViewerExt 이름 사용)
    let handler_paths = [
        "Software\\Classes\\*\\shellex\\ContextMenuHandlers\\UriViewerExt",
        "Software\\Classes\\SystemFileAssociations\\image\\shellex\\ContextMenuHandlers\\UriViewerExt",
    ];

    for path in handler_paths {
        set_reg_value(path, "", clsid_str)?;
    }

    Ok(())
}

fn delete_reg_key(key_path: &str) -> Result<(), String> {
    unsafe {
        let wide_key: Vec<u16> = key_path.encode_utf16().chain(std::iter::once(0)).collect();
        let _ = RegDeleteTreeW(HKEY_CURRENT_USER, PCWSTR(wide_key.as_ptr()));
    }
    Ok(())
}

fn set_reg_value(key_path: &str, value_name: &str, value_data: &str) -> Result<(), String> {
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
            return Err(format!("Failed to create/open registry key: {:?}", status));
        }

        let wide_name: Vec<u16> = value_name
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let wide_data: Vec<u16> = value_data
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let status = RegSetValueExW(
            hkey,
            PCWSTR(if value_name.is_empty() {
                ptr::null()
            } else {
                wide_name.as_ptr()
            }),
            0,
            REG_SZ,
            Some(std::slice::from_raw_parts(
                wide_data.as_ptr() as *const u8,
                wide_data.len() * 2,
            )),
        );

        if status.is_err() {
            return Err(format!("Failed to set registry value: {:?}", status));
        }
    }
    Ok(())
}

// ── Registry Helper ──
fn get_reg_value(key_path: &str, value_name: &str) -> Result<String, String> {
    use windows::Win32::System::Registry::{
        RegOpenKeyExW, RegQueryValueExW, HKEY_CURRENT_USER, KEY_READ,
    };

    unsafe {
        let wide_key: Vec<u16> = key_path.encode_utf16().chain(std::iter::once(0)).collect();
        let mut hkey = HKEY::default();

        let status = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(wide_key.as_ptr()),
            0,
            KEY_READ,
            &mut hkey,
        );

        if status.is_err() {
            return Err("Key not found".into());
        }

        let wide_name: Vec<u16> = value_name
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let mut value_type = windows::Win32::System::Registry::REG_VALUE_TYPE(0);
        let mut data_len = 0u32;

        let mut status = RegQueryValueExW(
            hkey,
            PCWSTR(if value_name.is_empty() {
                ptr::null()
            } else {
                wide_name.as_ptr()
            }),
            None,
            Some(&mut value_type),
            None,
            Some(&mut data_len),
        );

        if status.is_err() || data_len == 0 {
            return Err("Value not found".into());
        }

        let mut data = vec![0u8; data_len as usize];
        status = RegQueryValueExW(
            hkey,
            PCWSTR(if value_name.is_empty() {
                ptr::null()
            } else {
                wide_name.as_ptr()
            }),
            None,
            Some(&mut value_type),
            Some(data.as_mut_ptr()),
            Some(&mut data_len),
        );

        if status.is_ok() {
            let wide_data = std::slice::from_raw_parts(data.as_ptr() as *const u16, data.len() / 2);
            let mut s = String::from_utf16_lossy(wide_data);
            if let Some(pos) = s.find('\0') {
                s.truncate(pos);
            }
            Ok(s)
        } else {
            Err("Failed to query value".into())
        }
    }
}
