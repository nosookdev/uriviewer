// tray.rs — System tray management

use tray_icon::{TrayIconBuilder, TrayIcon};
use muda::{Menu, MenuItem, PredefinedMenuItem};
use std::path::{Path, PathBuf};

pub fn create_tray_menu() -> Menu {
    let menu = Menu::new();
    let open_item = MenuItem::with_id("open", "열기", true, None);
    let capture_item = MenuItem::with_id("capture", "화면 캡처", true, None);
    let picker_item = MenuItem::with_id("picker", "컬러 피커", true, None);
    let quit_item = MenuItem::with_id("quit", "종료", true, None);

    let _ = menu.append(&open_item);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&capture_item);
    let _ = menu.append(&picker_item);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&quit_item);
    
    menu
}

pub fn create_tray(icon_path: Option<PathBuf>) -> TrayIcon {
    let menu = create_tray_menu();
    
    let mut builder = TrayIconBuilder::new()
        .with_tooltip("uriviewer")
        .with_menu(Box::new(menu));

    // Try to load icon from path, fallback to default if None or fails
    let path = icon_path.unwrap_or_else(|| PathBuf::from("assets/logo.png"));
    
    if path.exists() {
        if let Ok(icon) = load_tray_icon(&path) {
            builder = builder.with_icon(icon);
        }
    }

    builder.build().unwrap()
}

fn load_tray_icon(path: &Path) -> Result<tray_icon::Icon, String> {
    let img = image::open(path).map_err(|e| e.to_string())?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    tray_icon::Icon::from_rgba(rgba.as_raw().clone(), w, h).map_err(|e| e.to_string())
}
