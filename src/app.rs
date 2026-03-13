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
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgb(15, 15, 16); // --bg-base
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(26, 26, 29); // --bg-surface
        visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(152, 152, 168)); // --text-secondary
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(36, 36, 40); // --bg-elevated
        visuals.selection.bg_fill = egui::Color32::from_rgb(60, 60, 65); // Changed blue to Gray tone
        visuals.window_fill = egui::Color32::from_rgb(26, 26, 29);
        _cc.egui_ctx.set_visuals(visuals);

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
        };
        if let Some(p) = initial_path { app.open_image_path(&_cc.egui_ctx, &p); }
        app
    }

    fn open_image_path(&mut self, ctx: &Context, path: &Path) {
        self.folder_imgs = crate::nav::images_in_folder(path);
        self.folder_idx = crate::nav::current_index(&self.folder_imgs, path);
        if let Some(dir) = path.parent() {
            self.config.last_directory = Some(dir.to_path_buf());
            save_config(&self.config);
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

        ctx.input(|i| {
            if i.key_pressed(Key::Num0) { self.view_state.fit_mode = true; self.view_state.offset = Vec2::ZERO; }
            if i.key_pressed(Key::Num1) { self.view_state.fit_mode = false; self.view_state.scale = 1.0; self.view_state.offset = Vec2::ZERO; }
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
        
        let nav = ctx.input(|i| {
            if i.key_pressed(Key::ArrowLeft) { Some(-1i64) } 
            else if i.key_pressed(Key::ArrowRight) { Some(1i64) }
            else { None }
        });
        if let Some(d) = nav { self.navigate(ctx, d); }
    }

    fn zoom_by(&mut self, factor: f32) {
        let current = if self.view_state.fit_mode { 1.0 } else { self.view_state.scale };
        self.view_state.fit_mode = false;
        self.view_state.scale = (current * factor).clamp(0.02, 32.0);
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
        // Ensure Dark Mode visuals are applied robustly
        let mut visuals = egui::Visuals::dark();
        visuals.selection.bg_fill = egui::Color32::from_rgb(70, 70, 75); 
        visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(180, 180, 190));
        
        // Active (Selected) widget style
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(80, 80, 85);
        visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        visuals.widgets.active.rounding = egui::Rounding::same(4.0);
        
        // Inactive (Normal) widget style
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(25, 25, 28);
        visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(160, 160, 170));
        
        ctx.set_visuals(visuals);
        ctx.style_mut(|s| {
            s.spacing.button_padding = egui::vec2(10.0, 5.0); 
            s.spacing.item_spacing = egui::vec2(6.0, 0.0);
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

        // Process background image loading
        if let Some(rx) = &self.loading_rx {
            ctx.request_repaint(); // Force repaint for loading animation
            if let Ok(res) = rx.try_recv() {
                self.loading_rx = None;
                match res {
                    Ok(img) => {
                        self.view_state.reset();
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

        egui::TopBottomPanel::top("toolbar").exact_height(54.0).show(ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                self.render_toolbar(ui, ctx);
            });
        });

        egui::TopBottomPanel::bottom("statusbar").exact_height(30.0).show(ctx, |ui| {
            self.render_statusbar(ui);
        });

        if self.config.info_open {
            egui::SidePanel::right("info_panel")
                .resizable(true)
                .min_width(320.0)
                .default_width(320.0)
                .show(ctx, |ui| {
                    self.render_info_panel(ui);
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.view_mode {
                ViewMode::Viewer => self.render_viewer(ui, ctx),
                ViewMode::Gallery => self.render_gallery(ui, ctx),
            }
        });
    }
}

impl RustViewApp {
    fn render_toolbar(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        ui.spacing_mut().item_spacing = egui::vec2(12.0, 0.0);
        
        // --- Left Side ---
        // App Icon
        ui.add_space(4.0);
        ui.label(egui::RichText::new("🦀").size(18.0));
        
        // File Actions
        if ui.button("📂").on_hover_text("파일 열기 (Ctrl+O)").clicked() { self.open_dialog(ctx); }
        
        // Navigation
        if ui.button("‹").clicked() { self.navigate(ctx, -1); }
        if ui.button("›").clicked() { self.navigate(ctx, 1); }
        
        ui.separator();

        if ui.add(egui::Button::new("✂").selected(self.view_state.selection_mode)).on_hover_text("영역 선택 (S)").clicked() {
            self.view_state.selection_mode = !self.view_state.selection_mode;
            if !self.view_state.selection_mode { 
                self.view_state.selection = None; 
                self.status = None;
            } else {
                self.status = Some("영역 선택 모드 활성화".to_string());
                if self.view_mode == ViewMode::Gallery { self.view_mode = ViewMode::Viewer; }
            }
        }

        ui.separator();

        // Filename text variable
        let filename_to_show = self.image.as_ref().map(|img| {
            img.path.file_name().unwrap_or_default().to_string_lossy().to_string()
        });

        // --- Right Side (Order in R2L: Controls -> Tabs -> Rotate -> Zoom) ---
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(8.0);
            
            // 1. Far Right: Fullscreen, Info, Checker
            let is_fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
            if ui.add(egui::Button::new("⛶").selected(is_fullscreen)).on_hover_text("전체화면 (F11)").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fullscreen));
            }
            if ui.add(egui::Button::new("ℹ").selected(self.config.info_open)).on_hover_text("정보 패널 (I)").clicked() {
                self.config.info_open = !self.config.info_open;
            }
            if ui.add(egui::Button::new("▦").selected(self.view_state.checker)).on_hover_text("투명 배경 격자 (T)").clicked() {
                self.view_state.checker = !self.view_state.checker;
            }
            
            ui.separator();

            // 2. Rotate Controls
            if ui.button("⟳").on_hover_text("시계 방향 회전 (R)").clicked() { self.view_state.rotation = self.view_state.rotation.cw(); }
            if ui.button("⟲").on_hover_text("반시계 방향 회전 (L)").clicked() { self.view_state.rotation = self.view_state.rotation.ccw(); }

            ui.separator();

            // 3. View Tabs
            let mut mode = self.view_mode.clone();
            let viewer_selected = mode == ViewMode::Viewer;
            let gallery_selected = mode == ViewMode::Gallery;

            if ui.add(egui::Button::new(egui::RichText::new("뷰어").strong()).selected(viewer_selected)).clicked() { mode = ViewMode::Viewer; }
            if ui.add(egui::Button::new(egui::RichText::new("갤러리").strong()).selected(gallery_selected)).clicked() { mode = ViewMode::Gallery; }
            if mode != self.view_mode {
                if mode == ViewMode::Gallery { self.enter_gallery(); } else { self.view_mode = ViewMode::Viewer; }
            }

            ui.separator();

            // 4. Zoom Controls
            // 4. Zoom Controls
            if let Some(_img) = &self.image {
                let zoom_selected = !self.view_state.fit_mode;
                let fit_selected = self.view_state.fit_mode;

                // RTL Order: [+] -> [%] -> [-] -> [1:1] -> [맞춤]
                // Which results in LTR: [맞춤] [1:1] [-] [%] [+]

                if ui.button("+").on_hover_text("확대 (+/MouseWheel)").clicked() {
                    self.zoom_by(1.25);
                }

                let zoom_pct = if fit_selected {
                    "FIT".to_string()
                } else {
                    format!("{:.0}%", self.view_state.scale * 100.0)
                };
                ui.add(egui::Button::new(egui::RichText::new(zoom_pct).monospace().size(11.0)).selected(zoom_selected));

                if ui.button("-").on_hover_text("축소 (-/MouseWheel)").clicked() {
                    self.zoom_by(1.0 / 1.25);
                }

                // Add a small spacing between zoom value controls and mode toggles
                ui.add_space(4.0);

                if ui.add(egui::Button::new("1:1").selected(self.view_state.scale == 1.0 && !fit_selected)).clicked() { 
                    self.view_state.fit_mode = false; 
                    self.view_state.scale = 1.0; 
                    self.view_state.offset = Vec2::ZERO;
                }
                
                if ui.add(egui::Button::new("맞춤").selected(fit_selected)).clicked() { 
                    self.view_state.fit_mode = true; 
                    self.view_state.offset = Vec2::ZERO;
                }
            }

            // Render Filename in the remaining center space
            if let Some(name) = filename_to_show {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add(egui::Label::new(egui::RichText::new(name).color(egui::Color32::from_rgb(180, 180, 190))).truncate());
                });
            }
        });
    }

    fn render_statusbar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(15.0, 0.0);
            if let Some(img) = &self.image {
                let (w, h) = img.display_size(self.view_state.rotation);
                
                // Index
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(5.0, 0.0);
                    ui.label(egui::RichText::new("🖼").size(11.0).color(egui::Color32::from_rgb(200, 200, 210)));
                    ui.label(egui::RichText::new(format!("{} / {}", self.folder_idx + 1, self.folder_imgs.len())).size(11.0).color(egui::Color32::WHITE));
                });

                // Dimensions
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(5.0, 0.0);
                    ui.label(egui::RichText::new("📐").size(11.0).color(egui::Color32::from_rgb(200, 200, 210)));
                    ui.label(egui::RichText::new(format!("{} × {}", w, h)).size(11.0).color(egui::Color32::WHITE));
                });

                // Size
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(5.0, 0.0);
                    ui.label(egui::RichText::new("💾").size(11.0).color(egui::Color32::from_rgb(200, 200, 210)));
                    ui.label(egui::RichText::new(human_size(img.file_size)).size(11.0).color(egui::Color32::WHITE));
                });
            }
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(status) = &self.status {
                    ui.colored_label(egui::Color32::from_rgb(255, 120, 120), status);
                } else {
                    ui.label(egui::RichText::new("← → 탐색 | +/- 줌 | I 정보 | G 갤러리").size(10.0).color(egui::Color32::from_rgb(160, 160, 170)));
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
                self.view_state.fit_mode = false;
            }
        }

        let painter = ui.painter_at(available);
        if let Some(img) = &self.image {
            let (dw, dh) = img.display_size(self.view_state.rotation);
            let scale = if self.view_state.fit_mode {
                let s = fit_scale(available, dw as f32, dh as f32);
                s * 0.98 // Slightly smaller than available to provide margins
            } else {
                self.view_state.scale
            };

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
                                egui::Frame::none()
                                    .fill(Color32::from_rgb(32, 32, 36))
                                    .stroke(egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 65)))
                                    .rounding(8.0)
                                    .shadow(egui::epaint::Shadow {
                                        offset: egui::vec2(0.0, 4.0),
                                        blur: 12.0,
                                        spread: 0.0,
                                        color: Color32::from_black_alpha(180),
                                    })
                                    .inner_margin(4.0)
                                    .show(ui, |ui| {
                                        ui.set_min_size(toolbar_size);
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
                                            
                                            let btn_copy = egui::Button::new(egui::RichText::new("📋").size(14.0))
                                                .fill(egui::Color32::TRANSPARENT);
                                            if ui.add(btn_copy).on_hover_text("클립보드 복사 (Enter)").clicked() {
                                                self.copy_selection(active_rect, img_rect);
                                            }
                                            
                                            let btn_save = egui::Button::new(egui::RichText::new("💾").size(14.0))
                                                .fill(egui::Color32::TRANSPARENT);
                                            if ui.add(btn_save).on_hover_text("파일로 저장 (Ctrl+S)").clicked() {
                                                self.save_selection(active_rect, img_rect);
                                            }
                                            
                                            ui.add_space(2.0);
                                            ui.separator();
                                            ui.add_space(2.0);
                                            
                                            let btn_close = egui::Button::new(egui::RichText::new("✕").size(12.0).strong())
                                                .fill(egui::Color32::TRANSPARENT);
                                            if ui.add(btn_close).on_hover_text("선택 취소 (Esc)").clicked() {
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
                draw_custom_spinner(ui, available.center(), _ctx.input(|i| i.time));
            }

        } else if self.loading_rx.is_some() {
            let available = ui.available_rect_before_wrap();
            draw_custom_spinner(ui, available.center(), _ctx.input(|i| i.time));
        } else {
            painter.text(available.center(), egui::Align2::CENTER_CENTER, "이미지를 선택하세요", egui::FontId::proportional(16.0), Color32::GRAY);
        }
    }

    fn render_info_panel(&self, ui: &mut egui::Ui) {
        if let Some(img) = &self.image {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 12.0);
                
                // 1. Thumbnail Preview (Properly allocated)
                ui.vertical_centered(|ui| {
                    let aspect = img.orig_h as f32 / img.orig_w as f32;
                    let available_w = ui.available_width();
                    let thumb_h = (available_w * aspect).min(160.0);
                    
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(30, 30, 34))
                        .rounding(8.0)
                        .show(ui, |ui| {
                            ui.add(egui::Image::new(&img.texture).max_size(egui::vec2(available_w, thumb_h)));
                        });
                });

                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(8.0);

                    // 2. File Information Section
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("📂 파일 정보").size(11.0).strong().color(egui::Color32::from_rgb(108, 143, 255)));
                        ui.add_space(6.0);
                        render_info_row(ui, "파일명", &img.fs_metadata.name);
                        render_info_row(ui, "포맷", &img.format);
                        render_info_row(ui, "파일 크기", &human_size(img.fs_metadata.size));
                        render_info_row(ui, "해상도", &format!("{} × {}", img.fs_metadata.width, img.fs_metadata.height));
                        if let Some(d) = &img.fs_metadata.modified {
                            render_info_row(ui, "수정일", d);
                        }
                    });

                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // 3. EXIF Section
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("🔍 EXIF 데이터").size(11.0).strong().color(egui::Color32::from_rgb(108, 143, 255)));
                        ui.add_space(6.0);
                        if let Some(exif) = &img.exif {
                            if let Some(v) = &exif.camera_model { render_info_row(ui, "카메라", v); }
                            if let Some(v) = &exif.lens { render_info_row(ui, "렌즈", v); }
                            if let Some(v) = &exif.exposure { render_info_row(ui, "노출", v); }
                            if let Some(v) = &exif.f_number { render_info_row(ui, "조리개", v); }
                            if let Some(v) = &exif.iso { render_info_row(ui, "ISO", v); }
                            if let Some(v) = &exif.focal_length { render_info_row(ui, "초점 거리", v); }
                        } else {
                            ui.label(egui::RichText::new("EXIF 정보가 없습니다.").size(12.0).color(egui::Color32::from_rgb(100, 100, 110)));
                        }
                    });

                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // 4. Folder/Location Section
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("📁 폴더").size(11.0).strong().color(egui::Color32::from_rgb(108, 143, 255)));
                        ui.add_space(6.0);
                        render_info_row(ui, "위치", &img.fs_metadata.location);
                        render_info_row(ui, "인덱스", &format!("{} / {}", self.folder_idx + 1, self.folder_imgs.len()));
                    });
                });
            });
        }
    }
}

fn render_info_row(ui: &mut egui::Ui, key: &str, val: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(key).size(12.0).color(egui::Color32::from_rgb(160, 160, 175)));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add(egui::Label::new(egui::RichText::new(val).size(12.0).strong().color(egui::Color32::from_rgb(245, 245, 250))).truncate());
        });
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn fit_scale(available: Rect, w: f32, h: f32) -> f32 {
    if w == 0.0 || h == 0.0 { return 1.0; }
    (available.width() / w).min(available.height() / h).min(1.0)
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

fn draw_custom_spinner(ui: &mut egui::Ui, center: Pos2, time: f64) {
    let painter = ui.painter();
    
    // Draw a dark semi-transparent backdrop to ensure visibility on light images
    let bg_radius = 32.0;
    painter.circle_filled(center, bg_radius, Color32::from_black_alpha(150));
    
    let radius = 18.0;
    let thickness = 4.0;
    let speed = 5.0; // rotation speed
    
    let angle = (time * speed) as f32 % std::f32::consts::TAU;
    
    // Primary spinning arc (light blue)
    painter.add(egui::Shape::Path(egui::epaint::PathShape {
        points: (0..=30).map(|i| {
            let a = angle + (i as f32 / 30.0) * std::f32::consts::PI; // Half circle
            center + Vec2::new(a.cos(), a.sin()) * radius
        }).collect(),
        closed: false,
        fill: Color32::TRANSPARENT,
        stroke: egui::Stroke::new(thickness, Color32::from_rgb(108, 143, 255)).into(),
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
    painter.rect_stroke(rect.expand(0.8), 0.0, egui::Stroke::new(1.0, Color32::from_black_alpha(80)));
    
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
