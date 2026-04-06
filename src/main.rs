// main.rs — Lightweight multi-format image viewer
// Entry point: parse CLI args, launch eframe window

#![windows_subsystem = "windows"]

use uriviewer_ext::app::RustViewApp;
use uriviewer_ext::win_utils;

use std::path::PathBuf;

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
        Box::new(move |cc| {
            Ok(Box::new(RustViewApp::new(cc, initial_path)))
        }),
    )
}

fn check_and_register_if_needed() -> Result<(), String> {
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let dll_path = exe_path.with_file_name("uriviewer_ext.dll");

    // DLL이 없으면 등록 불가
    if !dll_path.exists() {
        return Ok(());
    }

    // 개발 중 매번 재등록 (설정에서 끌 수 있게 하기 전까지 유지)
    win_utils::unregister_shell_extension();
    win_utils::register_shell_extension(&dll_path)?;
    
    Ok(())
}
