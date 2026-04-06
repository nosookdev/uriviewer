// app.rs — Main application state and rendering

use egui::{Color32, Context, Key, Painter, Pos2, Rect, Vec2};
use std::path::{Path, PathBuf};

use crate::loader::{human_size, load_image};
use crate::nav::is_image;
use crate::types::*;
use arboard::Clipboard;
use image::GenericImageView;

// ─────────────────────────────────────────────────────────────────────────────
// App state
// ─────────────────────────────────────────────────────────────────────────────

pub struct RustViewApp {
    pub(crate) image: Option<LoadedImage>,
    pub(crate) view_mode: ViewMode,
    pub(crate) view_state: ViewState,
    pub(crate) gallery: Option<Gallery>,
    pub(crate) config: AppConfig,
    pub(crate) folder_imgs: Vec<PathBuf>,
    pub(crate) folder_idx: usize,
    pub(crate) status: Option<String>,
    pub(crate) thumb_rx: std::sync::mpsc::Receiver<(PathBuf, egui::ColorImage)>,
    pub(crate) thumb_tx: std::sync::mpsc::Sender<(PathBuf, egui::ColorImage)>,
    pub(crate) loading_rx: Option<std::sync::mpsc::Receiver<Result<LoadedImage, String>>>,
    pub(crate) settings_open: bool,
    pub(crate) help_open: bool,
    pub(crate) is_maximized: bool,
    // --- New fields ---
    pub(crate) tray: Option<tray_icon::TrayIcon>,
    pub(crate) hotkey_manager: Option<crate::hotkeys::HotKeyManager>,
    pub(crate) is_capturing: bool,
    pub(crate) is_picking: bool,
    pub(crate) captured_screens: Vec<crate::capture::CapturedScreen>,
    pub(crate) capture_textures: Vec<egui::TextureHandle>,
    pub(crate) recording_hotkey: Option<HotkeyTarget>,
    pub(crate) hotkey_error: Option<String>,
    pub(crate) active_settings_tab: String,
    pub(crate) first_frame: bool,
}

#[derive(PartialEq, Copy, Clone)]
pub enum HotkeyTarget {
    Capture,
    ColorPicker,
}

impl RustViewApp {
    pub fn new(_cc: &eframe::CreationContext, initial_path: Option<PathBuf>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        if let Ok(data) = std::fs::read("C:/Windows/Fonts/malgun.ttf") {
            fonts.font_data.insert("malgun".to_owned(), std::sync::Arc::new(egui::FontData::from_owned(data)));
            fonts.families.entry(egui::FontFamily::Proportional).or_default().push("malgun".to_owned());
            fonts.families.entry(egui::FontFamily::Monospace).or_default().push("malgun".to_owned());
        }
        _cc.egui_ctx.set_fonts(fonts);

        let config = load_config();
        let (tx, rx) = std::sync::mpsc::channel();
        let mut app = Self {
            image: None,
            view_mode: ViewMode::Viewer,
            view_state: ViewState { checker: config.checker, ..Default::default() },
            gallery: None,
            config,
            folder_imgs: vec![],
            folder_idx: 0,
            status: None,
            thumb_rx: rx,
            thumb_tx: tx,
            loading_rx: None,
            settings_open: false,
            help_open: false,
            is_maximized: false,
            tray: None,
            hotkey_manager: None,
            is_capturing: false,
            is_picking: false,
            captured_screens: vec![],
            capture_textures: vec![],
            recording_hotkey: None,
            hotkey_error: None,
            active_settings_tab: "일반 설정".to_string(),
            first_frame: true,
        };

        // 초기 테마 적용
        app.apply_theme(&_cc.egui_ctx);

        // Initialize Tray & Hotkeys
        let tray = crate::tray::create_tray(None);
        app.tray = Some(tray);

        let hkm = crate::hotkeys::HotKeyManager::new();
        if let Some((mods, key)) = app.config.capture_hotkey {
            if let Some(hk) = crate::hotkeys::create_hotkey(mods, key, 1) {
                if let Err(e) = hkm.register(hk) {
                    app.hotkey_error = Some(format!("화면 캡처 단축키 등록 실패: {}", e));
                }
            }
        }
        if let Some((mods, key)) = app.config.color_picker_hotkey {
            if let Some(hk) = crate::hotkeys::create_hotkey(mods, key, 2) {
                if let Err(e) = hkm.register(hk) {
                    let prev_err = app.hotkey_error.take().unwrap_or_default();
                    app.hotkey_error = Some(format!("{}\n컬러 피커 단축키 등록 실패: {}", prev_err, e).trim().to_string());
                }
            }
        }
        app.hotkey_manager = Some(hkm);

        if let Some(p) = initial_path { 
            app.open_image_path(&_cc.egui_ctx, &p); 
        } else {
            // No initial path: load Downloads folder by default
            if let Some(download_dir) = dirs::download_dir() {
                app.folder_imgs = crate::nav::images_in_folder(&download_dir);
                if !app.folder_imgs.is_empty() {
                    app.view_mode = ViewMode::Gallery;
                    app.gallery = Some(Gallery::new(download_dir, 0));
                }
            }
        }
        app
    }

    fn open_image_path(&mut self, ctx: &Context, path: &Path) {
        self.folder_imgs = crate::nav::images_in_folder(path);
        self.folder_idx = crate::nav::current_index(&self.folder_imgs, path);
        if let Some(dir) = path.parent() {
            self.config.last_directory = Some(dir.to_path_buf());
            save_config(&self.config);
        }
        self.view_state.reset(self.view_state.lock_scale);
        if !self.view_state.lock_scale {
            self.view_state.scale_mode = ScaleMode::Fill; // 기본값
        }
        self.start_loading_image(ctx, path);
        self.view_mode = ViewMode::Viewer;
    }

    fn start_loading_image(&mut self, ctx: &Context, path: &Path) {
        let (tx, rx) = std::sync::mpsc::channel();
        self.loading_rx = Some(rx);
        self.status = None;
        let ctx_clone = ctx.clone();
        let path_clone = path.to_path_buf();
        std::thread::spawn(move || {
            let res = load_image(&ctx_clone, &path_clone);
            let _ = tx.send(res);
            ctx_clone.request_repaint();
        });
    }

    fn open_dialog(&mut self, ctx: &Context) {
        let start_dir = self.config.last_directory.clone().unwrap_or_else(|| dirs::picture_dir().unwrap_or_default());
        let picked = rfd::FileDialog::new().set_directory(start_dir).pick_file();
        if let Some(path) = picked { self.open_image_path(ctx, &path); }
    }

    fn open_folder_dialog(&mut self, ctx: &Context) {
        let start_dir = self.config.last_directory.clone().unwrap_or_else(|| dirs::picture_dir().unwrap_or_default());
        let picked = rfd::FileDialog::new().set_directory(start_dir).pick_folder();
        if let Some(path) = picked {
            self.folder_imgs = crate::nav::images_in_folder(&path);
            if let Some(first) = self.folder_imgs.first().cloned() { self.open_image_path(ctx, &first); }
            self.config.last_directory = Some(path);
        }
    }

    fn navigate(&mut self, ctx: &Context, delta: i64) {
        if self.folder_imgs.is_empty() { return; }
        let len = self.folder_imgs.len() as i64;
        let idx = ((self.folder_idx as i64 + delta).rem_euclid(len)) as usize;
        let path = self.folder_imgs[idx].clone();
        self.folder_idx = idx;
        self.start_loading_image(ctx, &path);
    }

    fn enter_gallery(&mut self) {
        if self.folder_imgs.is_empty() { return; }
        let folder = self.folder_imgs[0].parent().unwrap_or(Path::new(".")).to_path_buf();
        self.gallery = Some(Gallery::new(folder, self.folder_idx));
        self.view_mode = ViewMode::Gallery;
    }

    fn handle_input(&mut self, ctx: &Context) {
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).filter(|p| is_image(p)).collect()
        });
        if let Some(path) = dropped.into_iter().next() { self.open_image_path(ctx, &path); }

        // --- Hotkey Recording Logic ---
        if let Some(target) = self.recording_hotkey {
            let (mods, key_code) = ctx.input(|i| {
                let mut m = 0u32;
                if i.modifiers.shift { m |= 1; }
                if i.modifiers.ctrl { m |= 2; }
                if i.modifiers.alt { m |= 4; }
                if i.modifiers.command { m |= 8; }
                
                // Find first pressed key
                let key = i.keys_down.iter().next().cloned();
                (m, key)
            });

            if let Some(k) = key_code {
                // Map egui::Key to winapi/global-hotkey style if possible
                // For POC, we'll just allow common keys or stop recording
                let raw_code = match k {
                    egui::Key::A => 0x41, egui::Key::B => 0x42, egui::Key::C => 0x43, egui::Key::D => 0x44,
                    egui::Key::E => 0x45, egui::Key::F => 0x46, egui::Key::G => 0x47, egui::Key::H => 0x48,
                    egui::Key::I => 0x49, egui::Key::J => 0x4A, egui::Key::K => 0x4B, egui::Key::L => 0x4C,
                    egui::Key::M => 0x4D, egui::Key::N => 0x4E, egui::Key::O => 0x4F, egui::Key::P => 0x50,
                    egui::Key::Q => 0x51, egui::Key::R => 0x52, egui::Key::S => 0x53, egui::Key::T => 0x54,
                    egui::Key::U => 0x55, egui::Key::V => 0x56, egui::Key::W => 0x57, egui::Key::X => 0x58,
                    egui::Key::Y => 0x59, egui::Key::Z => 0x5A,
                    
                    egui::Key::Num0 => 0x30, egui::Key::Num1 => 0x31, egui::Key::Num2 => 0x32, egui::Key::Num3 => 0x33,
                    egui::Key::Num4 => 0x34, egui::Key::Num5 => 0x35, egui::Key::Num6 => 0x36, egui::Key::Num7 => 0x37,
                    egui::Key::Num8 => 0x38, egui::Key::Num9 => 0x39,

                    egui::Key::F1 => 0x70, egui::Key::F2 => 0x71, egui::Key::F3 => 0x72, egui::Key::F4 => 0x73,
                    egui::Key::F5 => 0x74, egui::Key::F6 => 0x75, egui::Key::F7 => 0x76, egui::Key::F8 => 0x77,
                    egui::Key::F9 => 0x78, egui::Key::F10 => 0x79, egui::Key::F11 => 0x7A, egui::Key::F12 => 0x7B,
                    _ => 0,
                };

                if raw_code != 0 {
                    let prev_config = self.config.clone();
                    match target {
                        HotkeyTarget::Capture => self.config.capture_hotkey = Some((mods, raw_code)),
                        HotkeyTarget::ColorPicker => self.config.color_picker_hotkey = Some((mods, raw_code)),
                    }
                    // Re-register hotkey
                    let hk_id = match target {
                        HotkeyTarget::Capture => 1,
                        HotkeyTarget::ColorPicker => 2,
                    };
                    if let Some(hkm) = &self.hotkey_manager {
                        if let Some(hk) = crate::hotkeys::create_hotkey(mods, raw_code, hk_id) {
                            match hkm.register(hk) {
                                Ok(_) => self.hotkey_error = None,
                                Err(e) => {
                                    self.hotkey_error = Some(format!("단축키 등록 실패: {}. 이미 사용 중일 수 있습니다.", e));
                                    self.config = prev_config; // Revert
                                }
                            }
                        }
                    }
                    save_config(&self.config);
                    self.recording_hotkey = None;
                }
            }
            return; // Consume input during recording
        }

        ctx.input(|i| {
            if i.key_pressed(Key::Num0) { self.view_state.scale_mode = ScaleMode::Fit; self.view_state.scale = 1.0; self.view_state.offset = Vec2::ZERO; }
            if i.key_pressed(Key::Num1) { self.view_state.scale_mode = ScaleMode::Original; self.view_state.scale = 1.0; self.view_state.offset = Vec2::ZERO; }
            if i.key_pressed(Key::Num2) { self.view_state.scale_mode = ScaleMode::Fill; self.view_state.scale = 1.0; self.view_state.offset = Vec2::ZERO; }
            if i.key_pressed(Key::I) { self.config.info_open = !self.config.info_open; }
            if i.key_pressed(Key::T) { self.view_state.checker = !self.view_state.checker; self.config.checker = self.view_state.checker; }
            if i.key_pressed(Key::L) { self.view_state.rotation = self.view_state.rotation.ccw(); }
            if i.key_pressed(Key::R) { self.view_state.rotation = self.view_state.rotation.cw(); }
            // S key for selection mode - consume it to prevent double-triggering
            // Also check for 's' text event to handle Korean IME ('ㄴ')
            let mut s_pressed = i.key_pressed(Key::S);
            if !s_pressed {
                for event in &i.raw.events {
                    if let egui::Event::Text(t) = event {
                        if t == "s" || t == "S" || t == "ㄴ" {
                            s_pressed = true;
                            break;
                        }
                    }
                }
            }

            if s_pressed && !i.modifiers.any() {
                self.view_state.selection_mode = !self.view_state.selection_mode;
                if !self.view_state.selection_mode {
                    self.view_state.selection = None;
                    self.status = None;
                } else {
                    self.status = Some("영역 선택 모드 활성화".to_string());
                    // Auto-switch to Viewer if in Gallery
                    if self.view_mode == ViewMode::Gallery {
                        self.view_mode = ViewMode::Viewer;
                    }
                }
            }

            if i.key_pressed(Key::F11) { 
                let is_fs = i.viewport().fullscreen.unwrap_or(false);
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fs));
            }

            if i.modifiers.alt && i.key_pressed(Key::L) {
                self.view_state.lock_scale = !self.view_state.lock_scale;
                self.status = Some(if self.view_state.lock_scale { "배율 고정 활성화".to_string() } else { "배율 고정 해제".to_string() });
            }
        });

        let toggle_gallery = ctx.input(|i| i.key_pressed(Key::G));
        if toggle_gallery {
            match self.view_mode {
                ViewMode::Viewer => self.enter_gallery(),
                ViewMode::Gallery => self.view_mode = ViewMode::Viewer,
            }
        }

        let zoom_in = ctx.input(|i| i.key_pressed(Key::Plus));
        let zoom_out = ctx.input(|i| i.key_pressed(Key::Minus));
        if zoom_in { self.zoom_by(1.25); }
        if zoom_out { self.zoom_by(1.0 / 1.25); }

        // Mouse Wheel Zoom
        let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
        if scroll > 0.0 {
            self.zoom_by(1.1);
        } else if scroll < 0.0 {
            self.zoom_by(1.0 / 1.1);
        }
        
        let nav = ctx.input(|i| {
            if i.key_pressed(Key::ArrowLeft) { Some(-1i64) } 
            else if i.key_pressed(Key::ArrowRight) { Some(1i64) }
            else { None }
        });
        if let Some(d) = nav { self.navigate(ctx, d); }
    }

    fn zoom_by(&mut self, factor: f32) {
        // 이미지가 없거나 렌더링되지 않은 상태면 무시
        let current_scale = self.view_state.scale;
        
        // 줌을 조절하면 고정 모드(Fit/Fill)에서 자유 모드(Original)로 전환
        self.view_state.scale_mode = ScaleMode::Original;
        self.view_state.scale = (current_scale * factor).clamp(0.02, 32.0);
    }

    fn copy_selection(&mut self, screen_rect: Rect, img_rect: Rect) {
        println!("DEBUG: copy_selection called");
        if self.image.is_none() { 
            self.status = Some("❌ 복사 실패: 이미지 없음".to_string());
            return; 
        }
        
        self.status = Some("⏳ 이미지 크롭 중...".to_string());
        let res = self.get_cropped_image(screen_rect, img_rect);
        
        match res {
            Ok(cropped) => {
                self.status = Some("⏳ 클립보드 접근 중...".to_string());
                match Clipboard::new() {
                    Ok(mut clipboard) => {
                        let rgba = cropped.to_rgba8();
                        let (w, h) = rgba.dimensions();
                        let img_data = arboard::ImageData {
                            width: w as usize,
                            height: h as usize,
                            bytes: std::borrow::Cow::from(rgba.as_raw()),
                        };
                        if let Err(e) = clipboard.set_image(img_data) {
                            self.status = Some(format!("❌ 클립보드 전송 오류: {}", e));
                            println!("DEBUG: Clipboard set_image error: {}", e);
                        } else {
                            self.status = Some("✅ 선택 영역이 클립보드에 복사되었습니다.".to_string());
                            println!("DEBUG: Copy successful");
                        }
                    }
                    Err(e) => {
                        self.status = Some(format!("❌ 클립보드 초기화 실패: {}", e));
                        println!("DEBUG: Clipboard initialization error: {}", e);
                    }
                }
            }
            Err(e) => {
                self.status = Some(format!("❌ 이미지 크롭 오류: {}", e));
                println!("DEBUG: Crop error: {}", e);
            }
        }
    }

    fn save_selection(&mut self, screen_rect: Rect, img_rect: Rect) {
        println!("DEBUG: save_selection called");
        if let Some(img) = &self.image {
            self.status = Some("⏳ 이미지 처리 중...".to_string());
            let res = self.get_cropped_image(screen_rect, img_rect);
            match res {
                Ok(cropped) => {
                    let start_dir = self.config.last_directory.clone()
                        .or_else(|| dirs::picture_dir())
                        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
                    
                    let filename = format!("crop_{}", img.path.file_name().unwrap_or_default().to_string_lossy());
                    
                    self.status = Some("📂 저장 위치를 선택하세요...".to_string());
                    println!("DEBUG: Opening file dialog at {:?}", start_dir);
                    
                    // RFD FileDialog
                    let picked = rfd::FileDialog::new()
                        .set_directory(&start_dir)
                        .set_file_name(&filename)
                        .add_filter("PNG Image (*.png)", &["png"])
                        .add_filter("JPEG Image (*.jpg)", &["jpg", "jpeg"])
                        .save_file();
                    
                    if let Some(path) = picked {
                        println!("DEBUG: Path picked: {:?}", path);
                        self.status = Some(format!("💾 저장 중: {}...", path.file_name().unwrap_or_default().to_string_lossy()));
                        match cropped.save(&path) {
                            Ok(_) => {
                                self.status = Some(format!("✅ 성공적으로 저장되었습니다: {}", path.file_name().unwrap_or_default().to_string_lossy()));
                                println!("DEBUG: Save successful");
                                if let Some(parent) = path.parent() {
                                    self.config.last_directory = Some(parent.to_path_buf());
                                }
                            }
                            Err(e) => {
                                self.status = Some(format!("❌ 파일 저장 오류: {}", e));
                                println!("DEBUG: Save error: {}", e);
                            }
                        }
                    } else {
                        self.status = Some("ℹ️ 저장이 취소되었습니다.".to_string());
                        println!("DEBUG: Save cancelled");
                    }
                }
                Err(e) => {
                    self.status = Some(format!("❌ 이미지 크롭 오류: {}", e));
                    println!("DEBUG: Crop error: {}", e);
                }
            }
        }
    }

    fn render_capture_overlay(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        let available = ui.available_rect_before_wrap();
        let painter = ui.painter();
        
        // Find primary screen or first screen
        if let Some(tex) = self.capture_textures.first() {
            painter.image(tex.id(), available, Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0)), Color32::WHITE);
        }

        let pointer_pos = ctx.input(|i| i.pointer.hover_pos());
        
        if self.is_picking {
            // Color Picker Magnifier
            if let Some(pos) = pointer_pos {
                let rect = Rect::from_center_size(pos, Vec2::splat(120.0));
                painter.rect_filled(rect, 60.0, Color32::from_black_alpha(100)); // Circle bg
                
                // Draw Magnifier
                if let Some(screen) = self.captured_screens.first() {
                    let rx = (pos.x - available.min.x) / available.width();
                    let ry = (pos.y - available.min.y) / available.height();
                    let px = (rx * screen.width as f32) as i32;
                    let py = (ry * screen.height as f32) as i32;
                    
                    if px >= 0 && px < screen.width as i32 && py >= 0 && py < screen.height as i32 {
                        let color = screen.image.get_pixel(px as u32, py as u32);
                        let egui_color = Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
                        
                        // Center pixel highlight
                        painter.rect_filled(Rect::from_center_size(pos, Vec2::splat(10.0)), 0.0, egui_color);
                        painter.rect_stroke(Rect::from_center_size(pos, Vec2::splat(11.0)), 0.0, egui::Stroke::new(1.0, Color32::WHITE));
                        
                        let hex = format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2]);
                        painter.text(pos + Vec2::new(0.0, 70.0), egui::Align2::CENTER_CENTER, &hex, egui::FontId::proportional(14.0), Color32::WHITE);
                        
                        if ctx.input(|i| i.pointer.any_click()) {
                            if let Ok(mut cb) = Clipboard::new() {
                                let _ = cb.set_text(hex.clone());
                                self.status = Some(format!("색상이 복사되었습니다: {}", hex));
                                self.is_capturing = false;
                            }
                        }
                    }
                }
            }
        } else {
            // Area Selector
            if let Some(mut sel) = self.view_state.selection.clone() {
                if ctx.input(|i| i.pointer.primary_down()) {
                    if let Some(pos) = pointer_pos {
                        sel.end = pos;
                        self.view_state.selection = Some(sel.clone());
                    }
                } else if ctx.input(|i| i.pointer.primary_released()) {
                    // Finalize capture
                    let rect = Rect::from_two_pos(sel.start, sel.end);
                    if rect.width() > 1.0 && rect.height() > 1.0 {
                        // For now we just copy the primary screen part
                        if let Some(screen) = self.captured_screens.first() {
                            let rx = (rect.min.x - available.min.x) / available.width();
                            let ry = (rect.min.y - available.min.y) / available.height();
                            let rw = rect.width() / available.width();
                            let rh = rect.height() / available.height();
                            
                            let x = (rx * screen.width as f32) as u32;
                            let y = (ry * screen.height as f32) as u32;
                            let w = (rw * screen.width as f32) as u32;
                            let h = (rh * screen.height as f32) as u32;
                            
                            if w > 0 && h > 0 {
                                let cropped = image::imageops::crop_imm(&screen.image, x, y, w, h).to_image();
                                if let Ok(mut cb) = Clipboard::new() {
                                    let img_data = arboard::ImageData {
                                        width: w as usize,
                                        height: h as usize,
                                        bytes: std::borrow::Cow::from(cropped.as_raw()),
                                    };
                                    if let Err(e) = cb.set_image(img_data) {
                                        self.status = Some(format!("❌ 복사 오류: {}", e));
                                    } else {
                                        self.status = Some("✅ 캡처 영역이 클립보드에 복사되었습니다.".to_string());
                                    }
                                }
                            }
                        }
                    }
                    self.is_capturing = false;
                    self.view_state.selection = None;
                }
                
                let draw_rect = Rect::from_two_pos(sel.start, sel.end);
                draw_marching_ants(painter, draw_rect, ctx.input(|i| i.time));
            } else if ctx.input(|i| i.pointer.primary_pressed()) {
                if let Some(pos) = pointer_pos {
                    self.view_state.selection = Some(SelectionState { start: pos, end: pos, active: true });
                }
            }
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.is_capturing = false;
            self.status = Some("작업 취소".to_string());
        }
    }

    pub fn start_capture(&mut self, ctx: &Context, is_picking: bool) {
        self.is_capturing = true;
        self.is_picking = is_picking;
        self.captured_screens = crate::capture::capture_all_screens();
        let mut texs = vec![];
        for (i, screen) in self.captured_screens.iter().enumerate() {
            let name = format!("capture_{}", i);
            let color_img = egui::ColorImage::from_rgba_unmultiplied(
                [screen.width as usize, screen.height as usize],
                &screen.image,
            );
            texs.push(ctx.load_texture(name, color_img, egui::TextureOptions::LINEAR));
        }
        self.capture_textures = texs;
        
        if !self.is_maximized {
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
        }
        self.status = Some(if is_picking { "색상을 선택할 픽셀을 클릭하세요" } else { "캡처할 영역을 드래그하세요" }.to_string());
    }

    fn poll_system_events(&mut self, ctx: &Context) {
        if let Ok(event) = tray_icon::TrayIconEvent::receiver().try_recv() {
            match event {
                tray_icon::TrayIconEvent::DoubleClick { .. } => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                _ => {}
            }
        }

        if let Ok(event) = muda::MenuEvent::receiver().try_recv() {
            match event.id.as_ref() {
                "open" => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                "capture" => self.start_capture(ctx, false),
                "picker" => self.start_capture(ctx, true),
                "quit" => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                _ => {}
            }
        }

        if let Ok(event) = global_hotkey::GlobalHotKeyEvent::receiver().try_recv() {
            match event.id {
                1 => self.start_capture(ctx, false),
                2 => self.start_capture(ctx, true),
                _ => {}
            }
        }
    }

    fn get_cropped_image(&self, screen_rect: Rect, img_rect: Rect) -> Result<image::DynamicImage, String> {
        let img = self.image.as_ref().ok_or("No image")?;
        let mut full_img = image::open(&img.path).map_err(|e| e.to_string())?;
        
        // Apply rotation
        full_img = match self.view_state.rotation {
            Rotation::R90 => full_img.rotate90(),
            Rotation::R180 => full_img.rotate180(),
            Rotation::R270 => full_img.rotate270(),
            _ => full_img,
        };

        let (w, h) = full_img.dimensions();
        let rx = (screen_rect.min.x - img_rect.min.x) / img_rect.width();
        let ry = (screen_rect.min.y - img_rect.min.y) / img_rect.height();
        let rw = screen_rect.width() / img_rect.width();
        let rh = screen_rect.height() / img_rect.height();

        let x = (rx * w as f32).max(0.0) as u32;
        let y = (ry * h as f32).max(0.0) as u32;
        let width = (rw * w as f32).min((w - x) as f32) as u32;
        let height = (rh * h as f32).min((h - y) as f32) as u32;

        if width == 0 || height == 0 { return Err("Invalid dimensions".to_string()); }

        Ok(full_img.crop_imm(x, y, width, height))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// UI Rendering
// ─────────────────────────────────────────────────────────────────────────────

impl eframe::App for RustViewApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        if self.first_frame {
            self.apply_theme(ctx);
            self.view_state.theme_anim_start = Some(ctx.input(|i| i.time));
            self.first_frame = false;
        }
        
        ctx.style_mut(|s| {
            s.spacing.button_padding = egui::vec2(8.0, 4.0); 
            s.spacing.item_spacing = egui::vec2(6.0, 6.0);
            s.spacing.window_margin = egui::Margin::same(10.0);
        });

        // Global cursor for selection mode
        if self.view_state.selection_mode {
            ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
        }

        // Process background thumbnails
        while let Ok((path, color_img)) = self.thumb_rx.try_recv() {
            if let Some(gallery) = &mut self.gallery {
                let name = format!("thumb:{}", path.file_name().unwrap_or_default().to_string_lossy());
                let tex = ctx.load_texture(name, color_img, egui::TextureOptions::LINEAR);
                gallery.thumbs.insert(path, ThumbState::Loaded(tex));
            }
        }

        self.handle_input(ctx);
        self.poll_system_events(ctx);

        // Process background image loading
        if let Some(rx) = &self.loading_rx {
            ctx.request_repaint(); // Force repaint for loading animation
            if let Ok(res) = rx.try_recv() {
                self.loading_rx = None;
                match res {
                    Ok(img) => {
                        self.view_state.reset(self.view_state.lock_scale);
                        if let Some(exif) = &img.exif {
                            if let Some(n) = exif.orientation_num {
                                self.view_state.rotation = match n {
                                    3 => Rotation::R180,
                                    6 => Rotation::R90,
                                    8 => Rotation::R270,
                                    _ => Rotation::R0,
                                };
                            }
                        }
                        self.image = Some(img);
                        self.status = None;
                    }
                    Err(e) => { self.status = Some(format!("오류: {e}")); }
                }
            }
        }

        // 1. Title Bar (Top)
        let title_bar_color = ctx.style().visuals.window_fill;
        let title_frame = egui::Frame::none()
            .fill(title_bar_color)
            .stroke(egui::Stroke::NONE)
            .inner_margin(egui::Margin::ZERO);

        egui::TopBottomPanel::top("title_bar")
            .frame(title_frame)
            .exact_height(40.0) // 폰트 크기 상향에 따른 패널 높이 조정
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.centered_and_justified(|ui| {
                    self.render_title_bar(ui, ctx);
                });
            });

        // 2. Activity Bar (Left)
        let activity_bar_color = ctx.style().visuals.window_fill;
        let activity_frame = egui::Frame::none()
            .fill(activity_bar_color)
            .inner_margin(egui::Margin::symmetric(0.0, 10.0));

        egui::SidePanel::left("activity_bar")
            .frame(activity_frame)
            .exact_width(48.0)
            .resizable(false)
            .show(ctx, |ui| {
                self.render_activity_bar(ui, ctx);
            });

        // 3. Status Bar (Bottom)
        let status_bar_color = ctx.style().visuals.window_fill;
        let status_frame = egui::Frame::none()
            .fill(status_bar_color)
            .inner_margin(egui::Margin::symmetric(10.0, 2.0));

        egui::TopBottomPanel::bottom("statusbar").frame(status_frame).exact_height(22.0).show(ctx, |ui| {
            self.render_statusbar(ui);
        });

        // 4. Info Panel (Right)
        if self.config.info_open {
            let info_panel_color = ctx.style().visuals.window_fill;
            egui::SidePanel::right("info_panel")
                .resizable(true)
                .min_width(320.0)
                .default_width(320.0)
                .frame(egui::Frame::none().fill(info_panel_color).inner_margin(12.0))
                .show(ctx, |ui| {
                    self.render_info_panel(ui);
                });
        }

        // 5. Central Viewport
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(ctx.style().visuals.panel_fill)) 
            .show(ctx, |ui| {
                if self.is_capturing {
                    self.render_capture_overlay(ui, ctx);
                } else {
                    match self.view_mode {
                        ViewMode::Viewer => self.render_viewer(ui, ctx),
                        ViewMode::Gallery => self.render_gallery(ui, ctx),
                    }
                }
            });

        // Render Settings & Help Windows
        self.render_settings_window(ctx);
        self.render_help_window(ctx);

        // --- Custom Resize Handles for Borderless Window ---
        self.render_resize_handles(ctx);

        // --- Theme Transition Fade Effect ---
        if let Some(start_time) = self.view_state.theme_anim_start {
            let elapsed = ctx.input(|i| i.time) - start_time;
            let duration = 0.4;
            if elapsed < duration {
                let alpha = (1.0 - (elapsed / duration) as f32).powf(2.0);
                let color = if self.config.theme == AppTheme::Dark {
                    egui::Color32::from_black_alpha((alpha * 255.0) as u8)
                } else {
                    egui::Color32::from_white_alpha((alpha * 255.0) as u8)
                };
                egui::Area::new(egui::Id::new("theme_fade"))
                    .fixed_pos(egui::Pos2::ZERO)
                    .order(egui::Order::Foreground)
                    .interactable(false)
                    .show(ctx, |ui| {
                        let screen_rect = ctx.screen_rect();
                        ui.painter().rect_filled(screen_rect, 0.0, color);
                    });
                ctx.request_repaint();
            } else {
                self.view_state.theme_anim_start = None;
            }
        }
    }
}

impl RustViewApp {
    fn activity_button(&self, ui: &mut egui::Ui, icon: &str, active: bool) -> egui::Response {
        let btn_size = egui::vec2(48.0, 48.0);
        let (rect, response) = ui.allocate_at_least(btn_size, egui::Sense::click());
        
        let hovered = response.hovered();
        let clicked = response.clicked();
        
        // Background
        let bg_fill = if active || clicked {
            ui.visuals().selection.bg_fill
        } else if hovered {
            ui.visuals().widgets.hovered.bg_fill
        } else {
            egui::Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, 0.0, bg_fill);
        
        // Icon color
        let fg = if active || hovered {
             ui.visuals().widgets.active.fg_stroke.color
        } else { 
            ui.visuals().widgets.inactive.fg_stroke.color
        };
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, icon, egui::FontId::proportional(22.0), fg);
        
        // Active indicator
        if active {
            let indicator_rect = egui::Rect::from_min_max(
                rect.left_top(),
                egui::pos2(rect.left() + 2.0, rect.bottom())
            );
            ui.painter().rect_filled(indicator_rect, 0.0, ui.visuals().selection.bg_fill);
        }
        
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    fn render_resize_handles(&self, ctx: &egui::Context) {
        let screen_rect = ctx.screen_rect();
        let border_thickness = 5.0; // Slightly thicker for easier grab
        
        let pointer_pos = ctx.input(|i| i.pointer.hover_pos());
        if let Some(pos) = pointer_pos {
            // Priority 1: Bottom-Right Corner (MUST be first)
            let br_rect = egui::Rect::from_min_max(
                egui::pos2(screen_rect.right() - border_thickness * 2.0, screen_rect.bottom() - border_thickness * 2.0),
                egui::pos2(screen_rect.right(), screen_rect.bottom())
            );
            if br_rect.contains(pos) {
                ctx.set_cursor_icon(egui::CursorIcon::ResizeSouthEast);
                if ctx.input(|i| i.pointer.any_down()) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(egui::viewport::ResizeDirection::SouthEast));
                }
                return; // Early return to prevent side-edges from overriding
            }

            // Priority 2: Bottom Edge
            let bottom_rect = egui::Rect::from_min_max(
                egui::pos2(screen_rect.left() + border_thickness, screen_rect.bottom() - border_thickness),
                egui::pos2(screen_rect.right() - border_thickness * 2.0, screen_rect.bottom())
            );
            if bottom_rect.contains(pos) {
                ctx.set_cursor_icon(egui::CursorIcon::ResizeVertical);
                if ctx.input(|i| i.pointer.any_down()) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(egui::viewport::ResizeDirection::South));
                }
                return;
            }
            
            // Priority 3: Right Edge
            let right_rect = egui::Rect::from_min_max(
                egui::pos2(screen_rect.right() - border_thickness, screen_rect.top() + border_thickness),
                egui::pos2(screen_rect.right(), screen_rect.bottom() - border_thickness * 2.0)
            );
            if right_rect.contains(pos) {
                ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                if ctx.input(|i| i.pointer.any_down()) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(egui::viewport::ResizeDirection::East));
                }
            }
        }
    }

    fn render_activity_icon(&mut self, ui: &mut egui::Ui, icon: &str, text: &str, active: bool) -> egui::Response {
        let res = self.activity_button(ui, icon, active);
        if res.hovered() {
            // Use a custom Area for the tooltip to place it precisely to the right
            let pos = res.rect.right_center() + egui::vec2(8.0, 0.0);
            egui::Area::new(egui::Id::new(text))
                .fixed_pos(pos)
                .order(egui::Order::Tooltip)
                .interactable(false)
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.label(egui::RichText::new(text).size(12.0));
                    });
                });
        }
        res
    }

    fn render_activity_bar(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        ui.vertical_centered(|ui| {
            ui.add_space(8.0);
            
            if self.render_activity_icon(ui, "🖼", "뷰어 (V)", self.view_mode == ViewMode::Viewer).clicked() {
                self.view_mode = ViewMode::Viewer;
            }
            if self.render_activity_icon(ui, "▦", "갤러리 (G)", self.view_mode == ViewMode::Gallery).clicked() {
                self.enter_gallery();
            }
            if self.render_activity_icon(ui, "📂", "파일 열기 (Ctrl+O)", false).clicked() {
                self.open_dialog(ctx);
            }
            if self.render_activity_icon(ui, "✂", "영역 선택 (S)", self.view_state.selection_mode).clicked() {
                self.view_state.selection_mode = !self.view_state.selection_mode;
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                if self.render_activity_icon(ui, "?", "도움말", self.help_open).clicked() {
                    self.help_open = !self.help_open;
                }
                if self.render_activity_icon(ui, "⚙", "설정", self.settings_open).clicked() {
                    self.settings_open = !self.settings_open;
                }
            });
        });
    }

    fn render_title_bar(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        let (rect, response) = ui.allocate_at_least(ui.available_size(), egui::Sense::click_and_drag());
        
        // --- 1. Window Dragging & Double Click Support ---
        if response.dragged_by(egui::PointerButton::Primary) {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        if response.double_clicked() {
            let next_max = !self.is_maximized;
            self.is_maximized = next_max;
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(next_max));
            ctx.request_repaint(); // Force immediate icon update
        }

        // --- 2. Central Title Text (Drawn directly, no input blocking) ---
        let title = self.image.as_ref()
            .map(|img| img.path.file_name().unwrap_or_default().to_string_lossy().to_string())
            .unwrap_or_else(|| "uriviewer".to_string());
        
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            title,
            egui::FontId::proportional(14.5), // 타이틀 폰트 크기 상향
            ui.visuals().widgets.noninteractive.fg_stroke.color
        );

        // --- 3. UI Elements (Menus & Buttons) ---
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
                
                // Left side: Menus
                ui.add_space(6.0);
                ui.style_mut().spacing.button_padding = egui::vec2(8.0, 0.0);
                
                ui.menu_button(egui::RichText::new("File").size(14.0), |ui| {
                    if ui.button("열기 (Ctrl+O)").clicked() { self.open_dialog(ctx); ui.close_menu(); }
                    ui.separator();
                    if ui.button("종료").clicked() { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
                });
                ui.menu_button(egui::RichText::new("View").size(14.0), |ui| {
                    if ui.button("정보 패널 (I)").clicked() { self.config.info_open = !self.config.info_open; ui.close_menu(); }
                    if ui.button("격자 배경 (T)").clicked() { self.view_state.checker = !self.view_state.checker; ui.close_menu(); }
                    ui.separator();
                    if ui.button("전체화면 (F11)").clicked() { 
                        let is_fs = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fs));
                        ui.close_menu(); 
                    }
                });

                // Right side: Window Controls & Tools
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                    
                    // Windows Control Buttons (Far Right)
                    let btn_size = egui::vec2(35.0, 31.0);
                    
                    let close_res = ui.add(egui::Button::new(egui::RichText::new("×").size(18.0)).fill(egui::Color32::TRANSPARENT).min_size(btn_size));
                    if close_res.hovered() {
                        ui.painter().rect_filled(close_res.rect, 0.0, egui::Color32::from_rgb(232, 17, 35));
                        ui.painter().text(close_res.rect.center(), egui::Align2::CENTER_CENTER, "×", egui::FontId::proportional(18.0), egui::Color32::WHITE);
                    }
                    if close_res.clicked() { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
                    
                    let max_res = ui.add(egui::Button::new("").fill(egui::Color32::TRANSPARENT).min_size(btn_size));
                    let icon_size = 9.0;
                    let icon_rect = egui::Rect::from_center_size(max_res.rect.center(), egui::Vec2::splat(icon_size));
                    let stroke = egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.fg_stroke.color);
                    if self.is_maximized {
                        let offset = 1.2;
                        let b1 = egui::Rect::from_center_size(icon_rect.center() + egui::vec2(offset, -offset), egui::Vec2::splat(icon_size - 1.0));
                        let b2 = egui::Rect::from_center_size(icon_rect.center() + egui::vec2(-offset, offset), egui::Vec2::splat(icon_size - 1.0));
                        ui.painter().rect_stroke(b1, 0.0, stroke);
                        ui.painter().rect_filled(b2, 0.0, ui.visuals().window_fill);
                        ui.painter().rect_stroke(b2, 0.0, stroke);
                    } else {
                        ui.painter().rect_stroke(icon_rect, 0.0, stroke);
                    }

                    if max_res.clicked() {
                        self.is_maximized = !self.is_maximized;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(self.is_maximized));
                        ctx.request_repaint();
                    }
                    
                    if ui.add(egui::Button::new(egui::RichText::new("—").size(11.0)).fill(egui::Color32::TRANSPARENT).min_size(btn_size)).clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    }

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);

                    // Image Tools
                    if let Some(_img) = &self.image {
                        ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
                        
                        let tool_btn_style = |ui: &mut egui::Ui| {
                            ui.style_mut().spacing.button_padding = egui::vec2(4.0, 2.0);
                        };

                        ui.scope(|ui| {
                            tool_btn_style(ui);
                            let is_fit = self.view_state.scale_mode == ScaleMode::Fit;
                            if ui.selectable_label(is_fit, egui::RichText::new("맞춤").size(14.0))
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked() {
                                self.view_state.scale_mode = ScaleMode::Fit; 
                                self.view_state.scale = 1.0;
                                self.view_state.offset = egui::Vec2::ZERO;
                            }
                        });

                        ui.scope(|ui| {
                            tool_btn_style(ui);
                            let is_fill = self.view_state.scale_mode == ScaleMode::Fill;
                            if ui.selectable_label(is_fill, egui::RichText::new("꽉차게").size(14.0))
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked() {
                                self.view_state.scale_mode = ScaleMode::Fill;
                                self.view_state.scale = 1.0;
                                self.view_state.offset = egui::Vec2::ZERO;
                            }
                        });

                        ui.scope(|ui| {
                            tool_btn_style(ui);
                            let is_orig = self.view_state.scale_mode == ScaleMode::Original && self.view_state.scale == 1.0;
                            if ui.selectable_label(is_orig, egui::RichText::new("1:1").size(14.0))
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked() {
                                self.view_state.scale_mode = ScaleMode::Original; 
                                self.view_state.scale = 1.0; 
                                self.view_state.offset = egui::Vec2::ZERO;
                            }
                        });

                        ui.add_space(8.0);

                        if ui.button(egui::RichText::new("+").size(18.0))
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text("확대")
                            .clicked() { self.zoom_by(1.2); }
                        
                        let zoom_pct = match self.view_state.scale_mode {
                            ScaleMode::Fit => "FIT".to_string(),
                            ScaleMode::Fill => "FILL".to_string(),
                            ScaleMode::Original => format!("{:.0}%", self.view_state.scale * 100.0),
                        };
                        ui.label(egui::RichText::new(zoom_pct).monospace().size(12.0));
                        
                        if ui.button(egui::RichText::new("-").size(18.0))
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text("축소")
                            .clicked() { self.zoom_by(1.0 / 1.2); }
                        
                        ui.add_space(8.0);
                        // --- 비율 고정 버튼 ---
                        ui.scope(|ui| {
                            tool_btn_style(ui);
                            let is_locked = self.view_state.lock_scale;
                            let lock_icon = if is_locked { "🔗" } else { "🔓" };
                            if ui.selectable_label(is_locked, egui::RichText::new(lock_icon).size(18.0))
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .on_hover_text(if is_locked { "배율 고정 해제" } else { "현재 배율 고정" })
                                .clicked() {
                                self.view_state.lock_scale = !is_locked;
                            }
                        });

                        ui.add_space(8.0);

                        if ui.button(egui::RichText::new("⟳").size(18.0))
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text("시계 방향 회전")
                            .clicked() { self.view_state.rotation = self.view_state.rotation.cw(); }
                        if ui.button(egui::RichText::new("⟲").size(18.0))
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text("반시계 방향 회전")
                            .clicked() { self.view_state.rotation = self.view_state.rotation.ccw(); }
                    }
                });
            });
        });
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        let mut visuals = match self.config.theme {
            AppTheme::Dark => {
                let mut v = egui::Visuals::dark();
                v.panel_fill = egui::Color32::from_rgb(30, 30, 30);
                v.window_fill = egui::Color32::from_rgb(37, 37, 38);
                v.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(25, 25, 25);
                v.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT; 
                v.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                v.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(217, 217, 217)); 
                v.widgets.hovered.bg_fill = egui::Color32::from_rgb(60, 60, 60);
                v.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                v.widgets.active.bg_fill = egui::Color32::from_rgb(80, 80, 80);
                v.widgets.active.bg_stroke = egui::Stroke::NONE;
                v.selection.bg_fill = egui::Color32::from_rgb(70, 70, 75);
                v
            }
            AppTheme::Light => {
                let mut v = egui::Visuals::light();
                v.panel_fill = egui::Color32::from_rgb(245, 245, 247); // Off-white
                v.window_fill = egui::Color32::from_rgb(255, 255, 255);
                v.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(240, 240, 240);
                v.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                v.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                v.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 40, 45));
                v.widgets.hovered.bg_fill = egui::Color32::from_rgb(225, 225, 225);
                v.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                v.widgets.active.bg_fill = egui::Color32::from_rgb(210, 210, 210);
                v.widgets.active.bg_stroke = egui::Stroke::NONE;
                v.selection.bg_fill = egui::Color32::from_rgb(210, 210, 220);
                v
            }
        };
        
        let rounding = egui::Rounding::same(2.0); 
        visuals.widgets.noninteractive.rounding = rounding;
        visuals.widgets.inactive.rounding = rounding;
        visuals.widgets.hovered.rounding = rounding;
        visuals.widgets.active.rounding = rounding;
        
        ctx.set_visuals(visuals);
    }

    // --- Dynamic Theme Colors ---
    fn accent_color(&self) -> Color32 {
        if self.config.theme == AppTheme::Dark {
            Color32::from_rgb(108, 143, 255) // Light Blue
        } else {
            Color32::from_rgb(0, 103, 210) // Mid Blue
        }
    }

    fn text_main_color(&self) -> Color32 {
        if self.config.theme == AppTheme::Dark {
            Color32::from_rgb(245, 245, 250)
        } else {
            Color32::from_rgb(25, 25, 30)
        }
    }

    fn text_sub_color(&self) -> Color32 {
        if self.config.theme == AppTheme::Dark {
            Color32::from_rgb(160, 160, 175)
        } else {
            Color32::from_rgb(100, 100, 110)
        }
    }

    fn render_statusbar(&self, ui: &mut egui::Ui) {
        let text_color = if self.config.theme == AppTheme::Dark { Color32::WHITE } else { self.text_main_color() };
        ui.style_mut().visuals.override_text_color = Some(text_color);
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(12.0, 0.0);
            
            if let Some(img) = &self.image {
                let (w, h) = img.display_size(self.view_state.rotation);
                ui.label(egui::RichText::new(format!("{} / {}", self.folder_idx + 1, self.folder_imgs.len())).size(11.0));
                ui.label(egui::RichText::new(format!("{} × {}", w, h)).size(11.0));
                ui.label(egui::RichText::new(human_size(img.file_size)).size(11.0));
            }
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(status) = &self.status {
                    let status_color = if self.config.theme == AppTheme::Dark { Color32::WHITE } else { self.accent_color() };
                    ui.add(egui::Label::new(egui::RichText::new(status).size(11.0).strong().color(status_color)).selectable(false));
                }
            });
        });
    }

    fn render_gallery(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        if self.folder_imgs.is_empty() {
             ui.centered_and_justified(|ui| { ui.label("갤러리에 표시할 이미지가 없습니다."); });
             return;
        }

        let mut gallery = self.gallery.take().unwrap_or_else(|| {
             let folder = self.folder_imgs[0].parent().unwrap_or(Path::new(".")).to_path_buf();
             Gallery::new(folder, self.folder_idx)
        });

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(10.0, 15.0);
            ui.horizontal_wrapped(|ui| {
                let thumb_size = self.config.thumb_size;
                for i in 0..self.folder_imgs.len() {
                    let path = self.folder_imgs[i].clone();
                    
                    let thumb = gallery.thumbs.entry(path.clone()).or_insert_with(|| {
                         let tx = self.thumb_tx.clone();
                         let p = path.clone();
                         std::thread::spawn(move || {
                             if let Ok(color_img) = crate::loader::decode_thumbnail(&p) {
                                 let _ = tx.send((p, color_img));
                             }
                         });
                         ThumbState::Loading
                    });

                    ui.allocate_ui(egui::vec2(thumb_size, thumb_size + 20.0), |ui| {
                        ui.vertical_centered(|ui| {
                            let response = match thumb {
                                ThumbState::Loaded(tex) => {
                                    let img = egui::Image::new(&*tex).max_size(Vec2::splat(thumb_size));
                                    ui.add(egui::ImageButton::new(img))
                                }
                                ThumbState::Loading => {
                                    ui.add_sized([thumb_size, thumb_size], egui::Spinner::new())
                                }
                                _ => {
                                    ui.add_sized([thumb_size, thumb_size], egui::Button::new("?"))
                                }
                            };

                            if response.clicked() {
                                self.folder_idx = i;
                                self.open_image_path(ctx, &path);
                            }
                            
                            let name = path.file_name().unwrap_or_default().to_string_lossy();
                            ui.add(egui::Label::new(egui::RichText::new(name.clone()).size(10.0)).truncate());
                        });
                    });
                }
            });
        });

        self.gallery = Some(gallery);
    }

    fn render_settings_window(&mut self, ctx: &Context) {
        let mut settings_open = self.settings_open;
        let mut request_close = false;
        egui::Window::new("⚙ 환경 설정")
            .open(&mut settings_open)
            .resizable(true)
            .default_size([700.0, 500.0])
            .collapsible(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // --- Sidebar ---
                    let sidebar_width = 150.0;
                    ui.allocate_ui(egui::vec2(sidebar_width, ui.available_height()), |ui| {
                        ui.vertical(|ui| {
                            ui.add_space(10.0);
                            let tabs = [
                                "일반 설정",
                                "탐색기 설정",
                                "보기 설정",
                                "연속 보기",
                                "연결 파일",
                                "작은이미지",
                            ];
                            for tab in tabs {
                                let is_active = self.active_settings_tab == tab;
                                let btn = egui::SelectableLabel::new(is_active, tab);
                                if ui.add_sized([sidebar_width - 10.0, 30.0], btn).clicked() {
                                    self.active_settings_tab = tab.to_string();
                                }
                                ui.add_space(2.0);
                            }
                        });
                    });

                    ui.separator();

                    // --- Main Content ---
                    ui.vertical(|ui| {
                        ui.add_space(10.0);
                        match self.active_settings_tab.as_str() {
                            "일반 설정" => {
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("갤러리 설정").strong());
                                    ui.add_space(4.0);
                                    ui.horizontal(|ui| {
                                        ui.label("썸네일 크기:");
                                        ui.add(egui::Slider::new(&mut self.config.thumb_size, 100.0..=300.0).suffix("px"));
                                    });
                                });
                                ui.add_space(8.0);
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("⌨ 전역 단축키 설정").strong());
                                    ui.add_space(4.0);
                                    self.render_hotkey_setting(ui, "화면 캡처", HotkeyTarget::Capture);
                                    ui.add_space(4.0);
                                    self.render_hotkey_setting(ui, "컬러 피커", HotkeyTarget::ColorPicker);
                                    if let Some(error) = &self.hotkey_error {
                                        ui.label(egui::RichText::new(error).color(egui::Color32::LIGHT_RED));
                                    }
                                });
                                ui.add_space(8.0);
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("🎨 테마 설정").strong());
                                    ui.add_space(4.0);
                                    ui.horizontal(|ui| {
                                        let mut theme = self.config.theme;
                                        if ui.radio_value(&mut theme, AppTheme::Dark, "다크 테마 (VS Code)").changed() {
                                            self.config.theme = theme;
                                            self.apply_theme(ctx);
                                        }
                                        if ui.radio_value(&mut theme, AppTheme::Light, "라이트 테마 (윈도우 표준)").changed() {
                                            self.config.theme = theme;
                                            self.apply_theme(ctx);
                                        }
                                    });
                                    ui.add_space(4.0);
                                    ui.label(egui::RichText::new("※ 테마 변경 시 조작부 가시성이 개선된 보더가 적용됩니다.").size(10.0).color(egui::Color32::from_gray(130)));
                                });
                            }
                            "탐색기 설정" => {
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("탐색기 컨텍스트 메뉴").strong());
                                    ui.add_space(4.0);
                                    if ui.checkbox(&mut self.config.use_shell_ext, "마우스 우클릭 메뉴에 UriViewer 추가").changed() {
                                        if self.config.use_shell_ext {
                                            let exe = std::env::current_exe().unwrap();
                                            let dll = exe.with_file_name("uriviewer_ext.dll");
                                            let _ = crate::win_utils::register_shell_extension(&dll);
                                        } else {
                                            crate::win_utils::unregister_shell_extension();
                                        }
                                    }
                                });
                            }
                            "보기 설정" => {
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("뷰어 렌더링").strong());
                                    ui.add_space(4.0);
                                    ui.checkbox(&mut self.view_state.checker, "투명 배경 격자 표시 (T)");
                                    ui.checkbox(&mut self.config.info_open, "정보 패널 항상 열기 (I)");
                                });
                            }
                            "연결 파일" => {
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("파일 확장자 연결").strong());
                                    ui.add_space(4.0);
                                    
                                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                                        ui.label("기본 프로그램으로 설정할 확장자를 선택하세요.");
                                        ui.add_space(5.0);
                                        
                                        let all_exts = crate::win_utils::get_all_extensions();
                                        egui::Grid::new("ext_grid").num_columns(5).spacing([20.0, 8.0]).show(ui, |ui| {
                                            for (i, ext) in all_exts.iter().enumerate() {
                                                let mut linked = self.config.associated_extensions.contains(ext);
                                                if ui.checkbox(&mut linked, ext).changed() {
                                                    if linked {
                                                        self.config.associated_extensions.insert(ext.clone());
                                                        let _ = crate::win_utils::register_file_association(ext);
                                                    } else {
                                                        self.config.associated_extensions.remove(ext);
                                                        crate::win_utils::unregister_file_association(ext);
                                                    }
                                                }
                                                if (i + 1) % 5 == 0 { ui.end_row(); }
                                            }
                                        });
                                    });
                                    
                                    ui.separator();
                                    ui.horizontal(|ui| {
                                        if ui.button("모두 선택(A)").clicked() {
                                            let all = crate::win_utils::get_all_extensions();
                                            for ext in all {
                                                self.config.associated_extensions.insert(ext.clone());
                                                let _ = crate::win_utils::register_file_association(&ext);
                                            }
                                        }
                                        if ui.button("모두 해제(U)").clicked() {
                                            let all = crate::win_utils::get_all_extensions();
                                            for ext in all {
                                                self.config.associated_extensions.remove(&ext);
                                                crate::win_utils::unregister_file_association(&ext);
                                            }
                                        }
                                    });
                                    ui.checkbox(&mut self.config.show_assoc_notification, "연결 파일 설정 변경시 알림 보기");
                                });
                            }
                            _ => {
                                ui.label("준비 중인 페이지입니다.");
                            }
                        }
                    });
                });

                ui.separator();
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    if ui.button("페이지 초기화(P)").clicked() {
                        // Reset current tab or all? For now, do nothing special
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("닫기").clicked() { request_close = true; }
                    });
                });
            });
        self.settings_open = settings_open && !request_close;
        if request_close {
            save_config(&self.config);
        }
    }

    fn render_hotkey_setting(&mut self, ui: &mut egui::Ui, label: &str, target: HotkeyTarget) {
        let hotkey = match target {
            HotkeyTarget::Capture => self.config.capture_hotkey,
            HotkeyTarget::ColorPicker => self.config.color_picker_hotkey,
        };

        ui.horizontal(|ui| {
            ui.label(format!("{}: ", label));
            
            let text = if self.recording_hotkey == Some(target) {
                "대기 중... (키를 누르세요)".to_string()
            } else {
                match hotkey {
                    Some((mods, key)) => format_hotkey(mods, key),
                    None => "미지정".to_string(),
                }
            };

            let btn_color = if self.recording_hotkey == Some(target) {
                egui::Color32::from_rgb(0, 122, 204)
            } else {
                egui::Color32::from_rgb(60, 60, 65)
            };

            let btn = egui::Button::new(egui::RichText::new(text).color(egui::Color32::WHITE)).fill(btn_color);
            if ui.add(btn).clicked() {
                self.recording_hotkey = Some(target);
                self.hotkey_error = None; // Clear error when starts recording
            }
        });
    }

    fn render_help_window(&mut self, ctx: &Context) {
        let mut help_open = self.help_open;
        let mut request_close = false;
        egui::Window::new("? 도움말")
            .open(&mut help_open)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("🦀 UriViewer").size(18.0).strong());
                        ui.label(egui::RichText::new("v0.1.0").color(egui::Color32::from_gray(120)));
                    });
                    ui.label("빠르고 가벼운 오픈소스 이미지 뷰어");
                });

                ui.add_space(8.0);

                ui.group(|ui| {
                    ui.label(egui::RichText::new("⌨ 단축키").strong());
                    ui.add_space(4.0);
                    egui::Grid::new("shortcuts_grid")
                        .num_columns(2)
                        .spacing([20.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("← / →"); ui.label("이전 / 다음 이미지"); ui.end_row();
                            ui.label("+ / -"); ui.label("확대 / 축소"); ui.end_row();
                            ui.label("Num 0"); ui.label("화면에 맞춤 (Fit)"); ui.end_row();
                            ui.label("Num 1"); ui.label("원본 크기 (1:1)"); ui.end_row();
                            ui.label("Num 2"); ui.label("화면에 꽉 채움 (Fill)"); ui.end_row();
                            ui.label("G"); ui.label("뷰어 / 갤러리 전환"); ui.end_row();
                            ui.label("I"); ui.label("정보 패널 토글"); ui.end_row();
                            ui.label("T"); ui.label("배경 격자 토글"); ui.end_row();
                            ui.label("L / R"); ui.label("회전 (반시계 / 시계)"); ui.end_row();
                            ui.label("S"); ui.label("영역 선택 모드"); ui.end_row();
                            ui.label("F11"); ui.label("전체화면"); ui.end_row();
                        });
                });

                ui.add_space(12.0);
                ui.vertical_centered(|ui| {
                    if ui.button("확인").clicked() {
                        request_close = true;
                    }
                });
            });
        self.help_open = help_open && !request_close;
    }

    fn render_viewer(&mut self, ui: &mut egui::Ui, _ctx: &Context) {
        let available = ui.available_rect_before_wrap();
        
        let mut selection_consumed = false;
        if self.view_state.selection_mode {
            // Check if pointer is over any UI area (like the floating toolbar) to avoid hijacking clicks
            let is_over_area = _ctx.is_pointer_over_area();
            let is_dragging = self.view_state.selection.as_ref().map_or(false, |s| s.active);
            
            if !is_over_area || is_dragging {
                let response = ui.allocate_rect(available, egui::Sense::drag());
                selection_consumed = true; 
                
                if response.drag_started() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        self.view_state.selection = Some(SelectionState {
                            start: pos,
                            end: pos,
                            active: true,
                        });
                    }
                } else if response.dragged() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        if let Some(sel) = &mut self.view_state.selection {
                            sel.end = pos;
                            sel.active = true;
                        }
                    }
                } else if response.drag_stopped() {
                    if let Some(sel) = &mut self.view_state.selection {
                        sel.active = false;
                        if sel.start.distance(sel.end) < 4.0 {
                            self.view_state.selection = None;
                        }
                    }
                }
            }
        }

        if !selection_consumed {
            let response = ui.allocate_rect(available, egui::Sense::click_and_drag());
            if response.dragged() {
                self.view_state.offset += response.drag_delta();
            }
        }

        let painter = ui.painter_at(available);
        if let Some(img) = &self.image {
            let (dw, dh) = img.display_size(self.view_state.rotation);
            let scale = match self.view_state.scale_mode {
                ScaleMode::Fit => {
                    // Aspect Fit: 전체가 보이고 경계에 딱 맞게 스케일 계산
                    fit_scale(available, dw as f32, dh as f32)
                }
                ScaleMode::Fill => {
                    // Aspect Fill: 유격 없이 꽉 채우기
                    let sw = available.width() / dw as f32;
                    let sh = available.height() / dh as f32;
                    sw.max(sh)
                }
                ScaleMode::Original => {
                    // 물리 픽셀 1:1 대응 (High DPI 고려)
                    1.0 / ui.ctx().pixels_per_point()
                }
            } * self.view_state.scale;

            let size = Vec2::new(dw as f32 * scale, dh as f32 * scale);
            let center = available.center() + self.view_state.offset;
            let img_rect = Rect::from_center_size(center, size);

            if self.view_state.checker {
                draw_checkerboard(&painter, img_rect.intersect(available));
            }

            // Implementation of rotation using Mesh
            let mut mesh = egui::Mesh::with_texture(img.texture.id());
            let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
            // Rotate UV coordinates based on current Rotation
            let (uv_tl, uv_tr, uv_br, uv_bl) = match self.view_state.rotation {
                Rotation::R0 => (uv.left_top(), uv.right_top(), uv.right_bottom(), uv.left_bottom()),
                Rotation::R90 => (uv.left_bottom(), uv.left_top(), uv.right_top(), uv.right_bottom()),
                Rotation::R180 => (uv.right_bottom(), uv.left_bottom(), uv.left_top(), uv.right_top()),
                Rotation::R270 => (uv.right_top(), uv.right_bottom(), uv.left_bottom(), uv.left_top()),
            };
            
            let pos_tl = img_rect.left_top();
            let pos_tr = img_rect.right_top();
            let pos_br = img_rect.right_bottom();
            let pos_bl = img_rect.left_bottom();

            mesh.vertices.push(egui::epaint::Vertex { pos: pos_tl, uv: uv_tl, color: Color32::WHITE });
            mesh.vertices.push(egui::epaint::Vertex { pos: pos_tr, uv: uv_tr, color: Color32::WHITE });
            mesh.vertices.push(egui::epaint::Vertex { pos: pos_br, uv: uv_br, color: Color32::WHITE });
            mesh.vertices.push(egui::epaint::Vertex { pos: pos_bl, uv: uv_bl, color: Color32::WHITE });
            
            mesh.indices.extend([0, 1, 2, 0, 2, 3]);
            painter.add(mesh);

            // --- Selection Mode Overlay ---
            if self.view_state.selection_mode {
                let badge_rect = Rect::from_center_size(
                    Pos2::new(available.center().x, available.top() + 40.0),
                    egui::vec2(200.0, 30.0)
                );
                painter.rect_filled(badge_rect, 15.0, Color32::from_black_alpha(180));
                painter.text(
                    badge_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "영역 선택 모드 (드래그)",
                    egui::FontId::proportional(14.0),
                    Color32::from_rgb(108, 143, 255)
                );
                
                // Also draw a thin blue border around the viewer area to show it's active
                painter.rect_stroke(available, 0.0, egui::Stroke::new(2.0, Color32::from_rgb(108, 143, 255)));
            }
            
            // --- Selection Rendering ---
            if let Some(sel) = &self.view_state.selection {
                let full_selection = Rect::from_two_pos(sel.start, sel.end).intersect(available);
                let active_rect = full_selection.intersect(img_rect);
                
                if full_selection.width() > 1.0 && full_selection.height() > 1.0 {
                    draw_marching_ants(&painter, full_selection, _ctx.input(|i| i.time));
                    _ctx.request_repaint(); // Animate marching ants
                    
                    // Floating Toolbar - Only if selection overlaps with image
                    if !sel.active && active_rect.width() > 4.0 && active_rect.height() > 4.0 {
                        // Keyboard Shortcuts using the active_rect (intersection with image)
                        let mut do_copy = false;
                        let mut do_save = false;
                        _ctx.input(|i| {
                            if i.key_pressed(Key::Enter) {
                                do_copy = true;
                            }
                            if i.key_pressed(Key::S) && i.modifiers.command {
                                do_save = true;
                            }
                        });
                        
                        if do_copy { self.copy_selection(active_rect, img_rect); }
                        if do_save { self.save_selection(active_rect, img_rect); }
                        
                        if _ctx.input(|i| i.key_pressed(Key::Escape)) {
                            self.view_state.selection = None;
                        }

                        // Ensure toolbar is within available area
                        let mut toolbar_pos = Pos2::new(active_rect.center().x - 65.0, active_rect.bottom() + 10.0);
                        let toolbar_size = egui::vec2(130.0, 34.0);
                        
                        // Keep within CentralPanel (available)
                        if toolbar_pos.x < available.left() + 4.0 { toolbar_pos.x = available.left() + 4.0; }
                        if toolbar_pos.x + toolbar_size.x > available.right() - 4.0 { toolbar_pos.x = available.right() - toolbar_size.x - 4.0; }
                        if toolbar_pos.y + toolbar_size.y > available.bottom() - 10.0 { 
                            toolbar_pos.y = active_rect.top() - toolbar_size.y - 10.0; 
                        }
                        if toolbar_pos.y < available.top() + 10.0 {
                            toolbar_pos.y = active_rect.bottom() + 10.0;
                        }

                        // Use egui::Area to ensure the toolbar is on top and handles its own clicks
                        egui::Area::new(egui::Id::new("selection_toolbar"))
                            .fixed_pos(toolbar_pos)
                            .order(egui::Order::Foreground)
                            .show(_ctx, |ui| {
                                let bg = if self.config.theme == AppTheme::Dark { Color32::from_rgb(45, 45, 50) } else { Color32::from_rgb(240, 240, 245) };
                                let stroke_color = if self.config.theme == AppTheme::Dark { Color32::from_rgb(70, 70, 75) } else { Color32::from_rgb(180, 180, 190) };
                                egui::Frame::none()
                                    .fill(bg)
                                    .stroke(egui::Stroke::new(1.0, stroke_color))
                                    .rounding(10.0)
                                    .shadow(egui::epaint::Shadow {
                                        offset: egui::vec2(0.0, 6.0),
                                        blur: 16.0,
                                        spread: 0.0,
                                        color: Color32::from_black_alpha(if self.config.theme == AppTheme::Dark { 200 } else { 80 }),
                                    })
                                    .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                                    .show(ui, |ui| {
                                        ui.set_min_size(toolbar_size);
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);
                                            
                                            if ui.button(egui::RichText::new("📋").size(14.0)).on_hover_text("클립보드 복사 (Enter)").clicked() {
                                                self.copy_selection(active_rect, img_rect);
                                            }
                                            
                                            if ui.button(egui::RichText::new("💾").size(14.0)).on_hover_text("파일로 저장 (Ctrl+S)").clicked() {
                                                self.save_selection(active_rect, img_rect);
                                            }
                                            
                                            ui.separator();
                                            
                                            if ui.button(egui::RichText::new("✕").size(12.0).strong()).on_hover_text("선택 취소 (Esc)").clicked() {
                                                self.view_state.selection = None;
                                            }
                                        })
                                    });
                            });
                    }
                }
            }
            
            // --- Navigation Overlays ---
            if self.folder_imgs.len() > 1 {
                let btn_size = egui::vec2(50.0, 80.0);
                let pointer_pos = _ctx.input(|i| i.pointer.hover_pos()).unwrap_or_default();
                
                // Left Prev Button
                let left_rect = Rect::from_center_size(
                    Pos2::new(available.left() + 40.0, available.center().y),
                    btn_size,
                );
                let left_hovered = left_rect.expand(20.0).contains(pointer_pos);
                let left_alpha = if left_hovered { 255 } else { 0 }; // Hidden until hover
                
                if left_alpha > 0 {
                    let text_color = Color32::from_white_alpha(left_alpha);
                    let left_btn = egui::Button::new(egui::RichText::new("‹").size(48.0).color(text_color))
                        .fill(Color32::from_black_alpha(left_alpha.saturating_sub(100)))
                        .frame(left_hovered);
                    if ui.put(left_rect, left_btn).clicked() {
                        self.navigate(_ctx, -1);
                    }
                }
                
                // Right Next Button
                let right_rect = Rect::from_center_size(
                    Pos2::new(available.right() - 40.0, available.center().y),
                    btn_size,
                );
                let right_hovered = right_rect.expand(20.0).contains(pointer_pos);
                let right_alpha = if right_hovered { 255 } else { 0 };
                
                if right_alpha > 0 {
                    let text_color = Color32::from_white_alpha(right_alpha);
                    let right_btn = egui::Button::new(egui::RichText::new("›").size(48.0).color(text_color))
                        .fill(Color32::from_black_alpha(right_alpha.saturating_sub(100)))
                        .frame(right_hovered);
                    if ui.put(right_rect, right_btn).clicked() {
                        self.navigate(_ctx, 1);
                    }
                }
            }

            // Show loading spinner if currently loading an image
            if self.loading_rx.is_some() {
                draw_custom_spinner(ui, available.center(), _ctx.input(|i| i.time), self.accent_color());
            }
 
        } else if self.loading_rx.is_some() {
            let available = ui.available_rect_before_wrap();
            draw_custom_spinner(ui, available.center(), _ctx.input(|i| i.time), self.accent_color());
        } else {
            painter.text(available.center(), egui::Align2::CENTER_CENTER, "이미지를 선택하세요", egui::FontId::proportional(16.0), Color32::GRAY);
        }
    }

    fn render_info_panel(&self, ui: &mut egui::Ui) {
        if let Some(img) = &self.image {
            let accent = self.accent_color();
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 12.0);
                
                // 1. Thumbnail Preview
                ui.vertical_centered(|ui| {
                    let aspect = img.orig_h as f32 / img.orig_w as f32;
                    let available_w = ui.available_width();
                    let thumb_h = (available_w * aspect).min(160.0);
                    
                    let bg = if self.config.theme == AppTheme::Dark { Color32::from_rgb(30, 30, 34) } else { Color32::from_rgb(230, 230, 235) };
                    egui::Frame::none()
                        .fill(bg)
                        .rounding(8.0)
                        .show(ui, |ui| {
                            ui.add(egui::Image::new(&img.texture).max_size(egui::vec2(available_w, thumb_h)));
                        });
                });
 
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(8.0);
 
                    // 2. File Information Section
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("📂 파일 정보").size(11.0).strong().color(accent));
                        ui.add_space(6.0);
                        self.render_info_row(ui, "파일명", &img.fs_metadata.name);
                        self.render_info_row(ui, "포맷", &img.format);
                        self.render_info_row(ui, "파일 크기", &human_size(img.fs_metadata.size));
                        self.render_info_row(ui, "해상도", &format!("{} × {}", img.fs_metadata.width, img.fs_metadata.height));
                        if let Some(d) = &img.fs_metadata.modified {
                            self.render_info_row(ui, "수정일", d);
                        }
                    });
 
                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(8.0);
 
                    // 3. EXIF Section
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("🔍 EXIF 데이터").size(11.0).strong().color(accent));
                        ui.add_space(6.0);
                        if let Some(exif) = &img.exif {
                            if let Some(v) = &exif.camera_model { self.render_info_row(ui, "카메라", v); }
                            if let Some(v) = &exif.lens { self.render_info_row(ui, "렌즈", v); }
                            if let Some(v) = &exif.exposure { self.render_info_row(ui, "노출", v); }
                            if let Some(v) = &exif.f_number { self.render_info_row(ui, "조리개", v); }
                            if let Some(v) = &exif.iso { self.render_info_row(ui, "ISO", v); }
                            if let Some(v) = &exif.focal_length { self.render_info_row(ui, "초점 거리", v); }
                        } else {
                            ui.label(egui::RichText::new("EXIF 정보가 없습니다.").size(12.0).color(self.text_sub_color()));
                        }
                    });
 
                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(8.0);
 
                    // 4. Folder/Location Section
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("📁 폴더").size(11.0).strong().color(accent));
                        ui.add_space(6.0);
                        self.render_info_row(ui, "위치", &img.fs_metadata.location);
                        self.render_info_row(ui, "인덱스", &format!("{} / {}", self.folder_idx + 1, self.folder_imgs.len()));
                    });
                });
            });
        }
    }
 
    fn render_info_row(&self, ui: &mut egui::Ui, key: &str, val: &str) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(key).size(12.0).color(self.text_sub_color()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(egui::Label::new(egui::RichText::new(val).size(12.0).strong().color(self.text_main_color())).truncate());
            });
        });
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn fit_scale(available: Rect, w: f32, h: f32) -> f32 {
    if w == 0.0 || h == 0.0 { return 1.0; }
    // 작은 이미지도 뷰어에 꽉 차게 확대하기 위해 1.0 제한 제거
    (available.width() / w).min(available.height() / h)
}

fn draw_checkerboard(painter: &Painter, rect: Rect) {
    const CELL: f32 = 12.0;
    let cols = ((rect.width() / CELL) as usize) + 1;
    let rows = ((rect.height() / CELL) as usize) + 1;
    let c1 = Color32::from_rgb(255, 255, 255);
    let c2 = Color32::from_rgb(230, 230, 230);
    for row in 0..rows {
        for col in 0..cols {
            let color = if (row + col) % 2 == 0 { c1 } else { c2 };
            let x = rect.left() + col as f32 * CELL;
            let y = rect.top() + row as f32 * CELL;
            let cell = Rect::from_min_size(Pos2::new(x, y), Vec2::splat(CELL)).intersect(rect);
            if cell.area() > 0.0 { painter.rect_filled(cell, 0.0, color); }
        }
    }
}

fn draw_custom_spinner(ui: &mut egui::Ui, center: Pos2, time: f64, accent: Color32) {
    let painter = ui.painter();
    
    // Draw a dark semi-transparent backdrop to ensure visibility on light images
    let bg_radius = 32.0;
    painter.circle_filled(center, bg_radius, Color32::from_black_alpha(150));
    
    let radius = 18.0;
    let thickness = 4.0;
    let speed = 5.0; // rotation speed
    
    let angle = (time * speed) as f32 % std::f32::consts::TAU;
    
    // Primary spinning arc (accent color)
    painter.add(egui::Shape::Path(egui::epaint::PathShape {
        points: (0..=30).map(|i| {
            let a = angle + (i as f32 / 30.0) * std::f32::consts::PI; // Half circle
            center + Vec2::new(a.cos(), a.sin()) * radius
        }).collect(),
        closed: false,
        fill: Color32::TRANSPARENT,
        stroke: egui::Stroke::new(thickness, accent).into(),
    }));

    // Secondary spinning arc (white) for contrast
    painter.add(egui::Shape::Path(egui::epaint::PathShape {
        points: (0..=30).map(|i| {
            let a = angle + std::f32::consts::PI + (i as f32 / 30.0) * (std::f32::consts::PI * 0.5); // Quarter circle on opposite side
            center + Vec2::new(a.cos(), a.sin()) * radius
        }).collect(),
        closed: false,
        fill: Color32::TRANSPARENT,
        stroke: egui::Stroke::new(thickness, Color32::WHITE).into(),
    }));
}

fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("rustview").join("config.toml"))
}

fn load_config() -> AppConfig {
    let Some(path) = config_path() else { return AppConfig::default(); };
    let Ok(text) = std::fs::read_to_string(&path) else { return AppConfig::default(); };
    toml::from_str(&text).unwrap_or_default()
}

fn save_config(config: &AppConfig) {
    let Some(path) = config_path() else { return; };
    if let Some(dir) = path.parent() { let _ = std::fs::create_dir_all(dir); }
    if let Ok(text) = toml::to_string(config) { let _ = std::fs::write(path, text); }
}

fn draw_marching_ants(painter: &Painter, rect: Rect, time: f64) {
    let speed = 30.0;
    let dash_len = 4.0;
    let offset = (time * speed) % (dash_len * 2.0);
    
    let stroke_white = egui::Stroke::new(1.2, Color32::WHITE);
    let stroke_black = egui::Stroke::new(1.2, Color32::BLACK);
    
    // Outer shadow/glow for visibility on any background
    painter.rect_stroke(rect.expand(1.0), 0.0, egui::Stroke::new(1.5, Color32::from_black_alpha(120)));
    painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, Color32::from_white_alpha(150)));
    
    // Draw edges
    let points = [
        rect.left_top(), rect.right_top(),
        rect.right_top(), rect.right_bottom(),
        rect.right_bottom(), rect.left_bottom(),
        rect.left_bottom(), rect.left_top(),
    ];
    
    for i in (0..points.len()).step_by(2) {
        let start = points[i];
        let end = points[i+1];
        let diff = end - start;
        let len = diff.length();
        let dir = diff / len;
        
        let mut d = -offset as f32;
        while d < len {
            let s = d.max(0.0);
            let e = (d + dash_len as f32).min(len);
            if e > s {
                painter.line_segment([start + dir * s, start + dir * e], stroke_white);
            }
            
            let s2 = (d + dash_len as f32).max(0.0);
            let e2 = (d + dash_len as f32 * 2.0).min(len);
            if e2 > s2 {
                painter.line_segment([start + dir * s2, start + dir * e2], stroke_black);
            }
            d += dash_len as f32 * 2.0;
        }
    }
}

fn format_hotkey(mods: u32, key: u32) -> String {
    let mut parts = vec![];
    if mods & 2 != 0 { parts.push("Ctrl"); }
    if mods & 1 != 0 { parts.push("Shift"); }
    if mods & 4 != 0 { parts.push("Alt"); }
    if mods & 8 != 0 { parts.push("Win"); }
    
    let key_str = match key {
        0x1F => "S",
        0x09 => "C",
        0x1E => "A",
        _ => "Unknown",
    };
    parts.push(key_str);
    parts.join("+")
}
