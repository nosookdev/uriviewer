// app.rs — Main application state and rendering

use std::path::{Path, PathBuf};
use egui::{Color32, Context, Key, Painter, Pos2, Rect, Vec2};

use crate::loader::{human_size, load_image, load_thumbnail};
use crate::nav::{current_index, images_in_folder, is_image};
use crate::types::*;

// ─────────────────────────────────────────────────────────────────────────────
// App state
// ─────────────────────────────────────────────────────────────────────────────

pub struct RustViewApp {
    image:      Option<LoadedImage>,
    view_mode:  ViewMode,
    view_state: ViewState,
    gallery:    Option<Gallery>,
    config:     AppConfig,
    // Cached folder list for viewer navigation
    folder_imgs: Vec<PathBuf>,
    folder_idx:  usize,
    // Error / status message
    status:     Option<String>,
}

impl RustViewApp {
    pub fn new(_cc: &eframe::CreationContext, initial_path: Option<PathBuf>) -> Self {
        // Korean (and CJK) font support via Malgun Gothic on Windows
        let mut fonts = egui::FontDefinitions::default();
        if let Ok(data) = std::fs::read("C:/Windows/Fonts/malgun.ttf") {
            fonts.font_data.insert(
                "malgun".to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(data)),
            );
            fonts.families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push("malgun".to_owned());
            fonts.families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push("malgun".to_owned());
        }
        _cc.egui_ctx.set_fonts(fonts);
        _cc.egui_ctx.set_visuals(egui::Visuals::dark());

        let config = load_config();

        let mut app = Self {
            image:       None,
            view_mode:   ViewMode::Viewer,
            view_state:  ViewState {
                checker: config.checker,
                ..Default::default()
            },
            gallery:     None,
            config,
            folder_imgs: vec![],
            folder_idx:  0,
            status:      None,
        };

        if let Some(p) = initial_path {
            app.open_image_path(&_cc.egui_ctx, &p);
        }

        app
    }

    // ── File opening ──────────────────────────────────────────────────────

    fn open_image_path(&mut self, ctx: &Context, path: &Path) {
        match load_image(ctx, path) {
            Ok(img) => {
                // Rebuild folder list
                self.folder_imgs = images_in_folder(path);
                self.folder_idx  = current_index(&self.folder_imgs, path);
                // Update config
                if let Some(dir) = path.parent() {
                    self.config.last_directory = Some(dir.to_path_buf());
                    save_config(&self.config);
                }
                self.view_state.reset();
                self.status = None;
                self.image  = Some(img);
                self.view_mode = ViewMode::Viewer;
            }
            Err(e) => {
                self.status = Some(format!("오류: {e}"));
            }
        }
    }

    fn open_dialog(&mut self, ctx: &Context) {
        let start_dir = self.config.last_directory.clone()
            .unwrap_or_else(|| dirs::picture_dir().unwrap_or_default());

        let picked = rfd::FileDialog::new()
            .set_directory(start_dir)
            .add_filter("이미지", &["jpg","jpeg","png","gif","bmp","webp","tif","tiff","ico"])
            .add_filter("모든 파일", &["*"])
            .pick_file();

        if let Some(path) = picked {
            self.open_image_path(ctx, &path);
        }
    }

    // ── Navigation ────────────────────────────────────────────────────────

    fn navigate(&mut self, ctx: &Context, delta: i64) {
        if self.folder_imgs.is_empty() { return; }
        let len = self.folder_imgs.len() as i64;
        let idx = ((self.folder_idx as i64 + delta).rem_euclid(len)) as usize;
        let path = self.folder_imgs[idx].clone();
        self.folder_idx = idx;
        match load_image(ctx, &path) {
            Ok(img) => {
                self.view_state.reset();
                self.image = Some(img);
                self.status = None;
            }
            Err(e) => self.status = Some(format!("오류: {e}")),
        }
    }

    // ── Gallery ───────────────────────────────────────────────────────────

    fn enter_gallery(&mut self) {
        if self.folder_imgs.is_empty() { return; }
        let folder = self.folder_imgs[0]
            .parent()
            .unwrap_or(Path::new("."))
            .to_path_buf();
        self.gallery = Some(Gallery::new(folder, self.folder_idx));
        self.view_mode = ViewMode::Gallery;
    }

    // ── Keyboard / input ──────────────────────────────────────────────────

    fn handle_input(&mut self, ctx: &Context) {
        // Drag-and-drop files
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw.dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .filter(|p| is_image(p))
                .collect()
        });
        if let Some(path) = dropped.into_iter().next() {
            self.open_image_path(ctx, &path);
        }

        ctx.input(|i| {
            // Navigation
            if i.key_pressed(Key::ArrowLeft)  { /* handled below with ctx */ }
            if i.key_pressed(Key::ArrowRight) { /* handled below with ctx */ }
            // Zoom
            if i.key_pressed(Key::Num0) {
                self.view_state.fit_mode = true;
                self.view_state.offset   = Vec2::ZERO;
            }
            if i.key_pressed(Key::Num1) {
                self.view_state.fit_mode = false;
                self.view_state.scale    = 1.0;
                self.view_state.offset   = Vec2::ZERO;
            }
            // Info panel
            if i.key_pressed(Key::I) {
                self.config.info_open = !self.config.info_open;
            }
            // Checkerboard
            if i.key_pressed(Key::T) {
                self.view_state.checker = !self.view_state.checker;
                self.config.checker     = self.view_state.checker;
            }
            // Rotation
            if i.key_pressed(Key::L) {
                self.view_state.rotation = self.view_state.rotation.ccw();
            }
            if i.key_pressed(Key::R) {
                self.view_state.rotation = self.view_state.rotation.cw();
            }
            // View toggle G
            if i.key_pressed(Key::G) { /* handled after borrow */ }
            // Fullscreen F
            if i.key_pressed(Key::F11) { /* toggle via ctx */ }
        });

        // Handle with mutable self after immutable input read
        let nav = ctx.input(|i| {
            let left  = i.key_pressed(Key::ArrowLeft)  || i.key_pressed(Key::PageUp);
            let right = i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::PageDown);
            if left  { Some(-1i64) }
            else if right { Some(1i64) }
            else { None }
        });
        if let Some(d) = nav {
            self.navigate(ctx, d);
        }

        let toggle_gallery = ctx.input(|i| i.key_pressed(Key::G));
        if toggle_gallery {
            match self.view_mode {
                ViewMode::Viewer  => self.enter_gallery(),
                ViewMode::Gallery => self.view_mode = ViewMode::Viewer,
            }
        }

        // Zoom via + / - keys
        let zoom_in  = ctx.input(|i| {
            i.key_pressed(Key::Plus)
        });
        let zoom_out = ctx.input(|i| i.key_pressed(Key::Minus));
        if zoom_in  { self.zoom_by(1.25); }
        if zoom_out { self.zoom_by(1.0 / 1.25); }
    }

    fn zoom_by(&mut self, factor: f32) {
        let current = if self.view_state.fit_mode {
            // We don't know the panel size here, so just disable fit mode
            // and use a reasonable scale
            1.0
        } else {
            self.view_state.scale
        };
        self.view_state.fit_mode = false;
        self.view_state.scale = (current * factor).clamp(0.02, 32.0);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// eframe::App implementation
// ─────────────────────────────────────────────────────────────────────────────

impl eframe::App for RustViewApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.handle_input(ctx);

        // ── Toolbar ───────────────────────────────────────────────────────
        egui::TopBottomPanel::top("toolbar")
            .exact_height(44.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.horizontal_centered(|ui| {
                    self.render_toolbar(ui, ctx);
                });
            });

        // ── Status bar ────────────────────────────────────────────────────
        egui::TopBottomPanel::bottom("statusbar")
            .exact_height(26.0)
            .show(ctx, |ui| {
                self.render_statusbar(ui);
            });

        // ── Info panel ────────────────────────────────────────────────────
        if self.config.info_open {
            egui::SidePanel::right("info_panel")
                .width_range(220.0..=300.0)
                .default_width(260.0)
                .frame(egui::Frame::default()
                    .fill(Color32::from_rgb(28, 28, 32))
                    .inner_margin(egui::Margin::same(8.0)))
                .show(ctx, |ui| {
                    self.render_info_panel(ui);
                });
        }

        // ── Main content ──────────────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(egui::Frame::none()
                .fill(Color32::from_rgb(15, 15, 16))) // --bg-base
            .show(ctx, |ui| {
                match self.view_mode.clone() {
                    ViewMode::Viewer  => self.render_viewer(ui, ctx),
                    ViewMode::Gallery => self.render_gallery(ui, ctx),
                }
            });
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Toolbar
// ─────────────────────────────────────────────────────────────────────────────

impl RustViewApp {
    fn render_toolbar(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        // App icon
        ui.label(egui::RichText::new("🦀").size(18.0));
        ui.add_space(2.0);

        // Open file
        if ui.button("📂").on_hover_text("파일 열기 (Ctrl+O)").clicked() {
            self.open_dialog(ctx);
        }

        separator(ui);

        // Prev / Next
        let has_imgs = self.folder_imgs.len() > 1;
        ui.add_enabled_ui(has_imgs, |ui| {
            if ui.button("‹").on_hover_text("이전 이미지 (←)").clicked() {
                self.navigate(ctx, -1);
            }
            if ui.button("›").on_hover_text("다음 이미지 (→)").clicked() {
                self.navigate(ctx, 1);
            }
        });

        separator(ui);

        // Filename
        if let Some(img) = &self.image {
            let name = img.path.file_name()
                .unwrap_or_default()
                .to_string_lossy();
            ui.label(
                egui::RichText::new(name.as_ref())
                    .color(Color32::from_rgb(240, 240, 242))
                    .strong()
            );
        } else {
            ui.label(
                egui::RichText::new("이미지 없음")
                    .color(Color32::from_rgb(85, 85, 95))
            );
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Fullscreen hint
            ui.label(egui::RichText::new("F11").color(Color32::from_rgb(85,85,95)).size(11.0));

            separator(ui);

            // View toggle
            let gallery_active = matches!(self.view_mode, ViewMode::Gallery);
            if ui.selectable_label(gallery_active,  "갤러리")
                .on_hover_text("갤러리 뷰 (G)").clicked() {
                if gallery_active {
                    self.view_mode = ViewMode::Viewer;
                } else {
                    self.enter_gallery();
                }
            }
            if ui.selectable_label(!gallery_active, "뷰어")
                .on_hover_text("뷰어 (G)").clicked() {
                self.view_mode = ViewMode::Viewer;
            }

            separator(ui);

            // Info panel toggle
            let info_btn = egui::RichText::new("ℹ").size(15.0);
            if ui.selectable_label(self.config.info_open, info_btn)
                .on_hover_text("정보 패널 (I)").clicked() {
                self.config.info_open = !self.config.info_open;
            }

            // Checkerboard toggle
            let checker_btn = egui::RichText::new("▦").size(13.0);
            if ui.selectable_label(self.view_state.checker, checker_btn)
                .on_hover_text("투명 배경 격자 (T)").clicked() {
                self.view_state.checker = !self.view_state.checker;
                self.config.checker     = self.view_state.checker;
            }

            separator(ui);

            // Rotation
            if ui.button("↺").on_hover_text("왼쪽 회전 (L)").clicked() {
                self.view_state.rotation = self.view_state.rotation.ccw();
            }
            if ui.button("↻").on_hover_text("오른쪽 회전 (R)").clicked() {
                self.view_state.rotation = self.view_state.rotation.cw();
            }

            separator(ui);

            // Zoom controls
            if ui.button("1:1").on_hover_text("원본 크기 (1)").clicked() {
                self.view_state.fit_mode = false;
                self.view_state.scale    = 1.0;
                self.view_state.offset   = Vec2::ZERO;
            }
            if ui.button("맞춤").on_hover_text("창에 맞춤 (0)").clicked() {
                self.view_state.fit_mode = true;
                self.view_state.offset   = Vec2::ZERO;
            }
            if ui.button("+").on_hover_text("확대 (+)").clicked() {
                self.zoom_by(1.25);
            }

            // Zoom percentage
            let pct = if self.view_state.fit_mode {
                "맞춤".to_string()
            } else {
                format!("{:.0}%", self.view_state.scale * 100.0)
            };
            ui.label(
                egui::RichText::new(pct)
                    .color(Color32::from_rgb(152, 152, 168))
                    .monospace()
                    .size(12.0)
            );

            if ui.button("−").on_hover_text("축소 (-)").clicked() {
                self.zoom_by(1.0 / 1.25);
            }
        });
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Viewer panel
// ─────────────────────────────────────────────────────────────────────────────

impl RustViewApp {
    fn render_viewer(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        let available = ui.available_rect_before_wrap();

        // Allocate the entire panel as interactive
        let response = ui.allocate_rect(
            available,
            egui::Sense::click_and_drag(),
        );

        // ── Mouse wheel zoom ──────────────────────────────────────────────
        let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
        if response.hovered() && scroll != 0.0 {
            let factor = 1.0 + scroll * 0.001;
            let current = if self.view_state.fit_mode {
                if let Some(img) = &self.image {
                    let (dw, dh) = img.display_size(self.view_state.rotation);
                    fit_scale(available, dw as f32, dh as f32)
                } else { 1.0 }
            } else {
                self.view_state.scale
            };
            self.view_state.fit_mode = false;
            self.view_state.scale    = (current * factor).clamp(0.02, 32.0);
        }

        // ── Drag pan ──────────────────────────────────────────────────────
        if response.dragged() {
            self.view_state.offset += response.drag_delta();
            self.view_state.fit_mode = false;
        }

        // ── Double-click to reset ─────────────────────────────────────────
        if response.double_clicked() {
            self.view_state.fit_mode = true;
            self.view_state.offset   = Vec2::ZERO;
        }

        let painter = ui.painter_at(available);

        if let Some(img) = &self.image {
            let (dw, dh) = img.display_size(self.view_state.rotation);

            // Calculate display rect
            let scale = if self.view_state.fit_mode {
                fit_scale(available, dw as f32, dh as f32)
            } else {
                self.view_state.scale
            };

            let size = Vec2::new(dw as f32 * scale, dh as f32 * scale);
            let center = available.center() + self.view_state.offset;
            let img_rect = Rect::from_center_size(center, size);
            // Clip to visible area for checkerboard
            let visible_rect = img_rect.intersect(available);

            // ── Checkerboard ──────────────────────────────────────────────
            if self.view_state.checker && !visible_rect.is_negative() {
                draw_checkerboard(&painter, visible_rect);
            }

            // ── Image (with rotation) ─────────────────────────────────────
            let rotation_rad = self.view_state.rotation.to_radians();
            if rotation_rad == 0.0 {
                painter.image(
                    img.texture.id(),
                    img_rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            } else {
                // Use egui::Image widget for rotation support
                let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
                // Adjust rect back to pre-rotation size for correct rendering
                let (raw_w, raw_h) = (img.orig_w as f32 * scale, img.orig_h as f32 * scale);
                let raw_rect = Rect::from_center_size(center, Vec2::new(raw_w, raw_h));
                egui::Image::new(egui::load::SizedTexture::new(
                    img.texture.id(),
                    [img.orig_w as f32, img.orig_h as f32],
                ))
                .rotate(rotation_rad, Vec2::splat(0.5))
                .paint_at(ui, raw_rect);
                let _ = (uv, img_rect); // suppress warnings
            }

            // ── Nav arrows ────────────────────────────────────────────────
            if self.folder_imgs.len() > 1 {
                let arrow_h = 64.0;
                let arrow_w = 36.0;
                let mid_y = available.center().y - arrow_h / 2.0;

                // Previous arrow
                let prev_rect = Rect::from_min_size(
                    Pos2::new(available.left() + 16.0, mid_y),
                    Vec2::new(arrow_w, arrow_h),
                );
                if nav_arrow(ui, prev_rect, "‹") {
                    self.navigate(ctx, -1);
                }

                // Next arrow
                let next_rect = Rect::from_min_size(
                    Pos2::new(available.right() - arrow_w - 16.0, mid_y),
                    Vec2::new(arrow_w, arrow_h),
                );
                if nav_arrow(ui, next_rect, "›") {
                    self.navigate(ctx, 1);
                }
            }

            // ── Zoom badge (bottom centre) ────────────────────────────────
            if response.hovered() {
                let pct_text = format!("{:.0}%", scale * 100.0);
                let badge_pos = Pos2::new(
                    available.center().x,
                    available.bottom() - 20.0,
                );
                painter.text(
                    badge_pos,
                    egui::Align2::CENTER_CENTER,
                    &pct_text,
                    egui::FontId::proportional(12.0),
                    Color32::from_rgba_unmultiplied(152, 152, 168, 200),
                );
            }
        } else {
            // Empty state
            painter.text(
                available.center(),
                egui::Align2::CENTER_CENTER,
                "이미지를 열거나 파일을 드롭하세요\n\n📂  파일 열기   |   단축키: ← → 탐색   +/- 줌   I 정보   G 갤러리",
                egui::FontId::proportional(14.0),
                Color32::from_rgb(85, 85, 95),
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Gallery panel
// ─────────────────────────────────────────────────────────────────────────────

impl RustViewApp {
    fn render_gallery(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        if self.folder_imgs.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("갤러리에 표시할 이미지가 없습니다.")
                        .color(Color32::from_rgb(85, 85, 95)),
                );
            });
            return;
        }

        // Folder bar
        egui::TopBottomPanel::top("gallery_folder_bar")
            .exact_height(32.0)
            .frame(egui::Frame::none()
                .fill(Color32::from_rgb(26, 26, 29))
                .inner_margin(egui::Margin::symmetric(12.0, 0.0)))
            .show_inside(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label("📁");
                    if let Some(gallery) = &self.gallery {
                        ui.label(
                            egui::RichText::new(gallery.folder.to_string_lossy().as_ref())
                                .monospace()
                                .size(11.0)
                                .color(Color32::from_rgb(85, 85, 95)),
                        );
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("{}개", self.folder_imgs.len()))
                                .size(11.0)
                                .color(Color32::from_rgb(85, 85, 95)),
                        );
                    });
                });
            });

        let thumb_size = self.config.thumb_size;
        let selected   = self.folder_idx;
        let images     = self.folder_imgs.clone();

        // Ensure gallery state exists
        if self.gallery.is_none() {
            self.enter_gallery();
        }

        // Batch thumbnail loading: max 2 per frame to avoid UI freeze
        let mut loads = 0usize;
        let mut has_pending = false;
        for path in &images {
            let in_map = self.gallery.as_ref().map_or(false, |g| g.thumbs.contains_key(path));
            if !in_map {
                if loads < 2 {
                    loads += 1;
                    let loaded = load_thumbnail(ctx, path);
                    if let Some(g) = &mut self.gallery {
                        match loaded {
                            Ok(tex) => { g.thumbs.insert(path.clone(), ThumbState::Loaded(tex)); }
                            Err(_)  => { g.thumbs.insert(path.clone(), ThumbState::Failed); }
                        }
                    }
                } else {
                    has_pending = true;
                    break;
                }
            }
        }
        if has_pending {
            ctx.request_repaint();
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(8.0);
                let cols = ((ui.available_width() - 16.0) / (thumb_size + 8.0)) as usize;
                let cols = cols.max(1);

                egui::Grid::new("gallery_grid")
                    .num_columns(cols)
                    .spacing([8.0, 8.0])
                    .min_col_width(thumb_size)
                    .show(ui, |ui| {
                        for (i, path) in images.iter().enumerate() {
                            let is_selected = i == selected;
                            let name = path.file_name()
                                .unwrap_or_default()
                                .to_string_lossy();

                            let sense = egui::Sense::click();
                            let (rect, response) = ui.allocate_exact_size(
                                Vec2::splat(thumb_size),
                                sense,
                            );

                            // Hover / select styling
                            let border_color = if is_selected {
                                Color32::from_rgb(108, 143, 255)
                            } else if response.hovered() {
                                Color32::from_rgba_unmultiplied(255, 255, 255, 30)
                            } else {
                                Color32::TRANSPARENT
                            };

                            let painter = ui.painter_at(rect);

                            // Thumbnail image
                            if let Some(g) = &self.gallery {
                                if let Some(ThumbState::Loaded(tex)) = g.thumbs.get(path) {
                                    // Fit thumbnail in square
                                    let tex_size = tex.size_vec2();
                                    let aspect = tex_size.x / tex_size.y;
                                    let (tw, th) = if aspect >= 1.0 {
                                        (thumb_size, thumb_size / aspect)
                                    } else {
                                        (thumb_size * aspect, thumb_size)
                                    };
                                    let img_rect = Rect::from_center_size(
                                        rect.center(),
                                        Vec2::new(tw, th),
                                    );
                                    painter.image(
                                        tex.id(),
                                        img_rect,
                                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                        Color32::WHITE,
                                    );
                                } else {
                                    // Placeholder
                                    painter.rect_filled(rect, 6.0, Color32::from_rgb(36, 36, 40));
                                    painter.text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        "🖼",
                                        egui::FontId::proportional(24.0),
                                        Color32::from_rgb(85, 85, 95),
                                    );
                                }
                            }

                            // Border
                            painter.rect_stroke(rect, 8.0, egui::Stroke::new(2.0, border_color));

                            // Hover label
                            if response.hovered() {
                                let label_rect = Rect::from_min_max(
                                    Pos2::new(rect.left(), rect.bottom() - 22.0),
                                    rect.max,
                                );
                                painter.rect_filled(label_rect, 0.0,
                                    Color32::from_rgba_unmultiplied(0, 0, 0, 160));
                                painter.text(
                                    Pos2::new(rect.center().x, rect.bottom() - 11.0),
                                    egui::Align2::CENTER_CENTER,
                                    name.as_ref(),
                                    egui::FontId::proportional(10.0),
                                    Color32::from_rgb(240, 240, 242),
                                );
                            }

                            // Click: open in viewer
                            if response.clicked() {
                                let p = path.clone();
                                self.folder_idx = i;
                                if let Some(g) = &mut self.gallery {
                                    g.selected = i;
                                }
                                self.open_image_path(ctx, &p);
                            }

                            // New row
                            if (i + 1) % cols == 0 {
                                ui.end_row();
                            }
                        }
                    });
                ui.add_space(8.0);
            });
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Info panel
// ─────────────────────────────────────────────────────────────────────────────

impl RustViewApp {
    fn render_info_panel(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(4.0);

                if let Some(img) = &self.image {
                    let name = img.path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                    let (dw, dh) = img.display_size(self.view_state.rotation);
                    let dim = Color32::from_rgb(160, 160, 180);
                    let val = Color32::WHITE;

                    // ── 파일 정보 ──
                    ui.label(egui::RichText::new("파일 정보").size(10.0).color(dim).strong());
                    ui.separator();
                    ui.label(egui::RichText::new(format!("파일명  {name}")).size(12.0).color(val));
                    ui.label(egui::RichText::new(format!("포맷    {}", img.format)).size(12.0).color(val));
                    ui.label(egui::RichText::new(format!("해상도  {dw} × {dh}")).size(12.0).color(val));
                    ui.label(egui::RichText::new(format!("크기    {}", human_size(img.file_size))).size(12.0).color(val));

                    // ── EXIF ──
                    if let Some(exif) = &img.exif {
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("EXIF").size(10.0).color(dim).strong());
                        ui.separator();
                        if let Some(v) = &exif.camera_make  { ui.label(egui::RichText::new(format!("제조사  {v}")).size(12.0).color(val)); }
                        if let Some(v) = &exif.camera_model { ui.label(egui::RichText::new(format!("카메라  {v}")).size(12.0).color(val)); }
                        if let Some(v) = &exif.lens         { ui.label(egui::RichText::new(format!("렌즈    {v}")).size(12.0).color(val)); }
                        if let Some(v) = &exif.exposure     { ui.label(egui::RichText::new(format!("셔터    {v}")).size(12.0).color(val)); }
                        if let Some(v) = &exif.f_number     { ui.label(egui::RichText::new(format!("조리개  {v}")).size(12.0).color(val)); }
                        if let Some(v) = &exif.iso          { ui.label(egui::RichText::new(format!("ISO     {v}")).size(12.0).color(val)); }
                        if let Some(v) = &exif.focal_length { ui.label(egui::RichText::new(format!("초점    {v}")).size(12.0).color(val)); }
                        if let Some(v) = &exif.date_taken   { ui.label(egui::RichText::new(format!("촬영일  {v}")).size(12.0).color(val)); }
                    }

                    // ── 폴더 ──
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("폴더").size(10.0).color(dim).strong());
                    ui.separator();
                    if let Some(dir) = img.path.parent() {
                        ui.label(egui::RichText::new(dir.to_string_lossy().as_ref())
                            .monospace().size(10.0).color(dim));
                    }
                    ui.label(egui::RichText::new(
                        format!("폴더 내  {} / {}", self.folder_idx + 1, self.folder_imgs.len())
                    ).size(12.0).color(val));
                } else {
                    ui.add_space(40.0);
                    ui.label(egui::RichText::new("이미지 없음").color(Color32::from_rgb(85, 85, 95)));
                }
            });
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Status bar
// ─────────────────────────────────────────────────────────────────────────────

impl RustViewApp {
    fn render_statusbar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(img) = &self.image {
                let (dw, dh) = img.display_size(self.view_state.rotation);
                status_item(ui, "🖼", &format!("{} / {}", self.folder_idx + 1, self.folder_imgs.len()));
                status_item(ui, "📐", &format!("{dw} × {dh}"));
                status_item(ui, "💾", &human_size(img.file_size));

                let zoom_str = if self.view_state.fit_mode {
                    "맞춤".to_string()
                } else {
                    format!("{:.0}%", self.view_state.scale * 100.0)
                };
                status_item(ui, "🔍", &zoom_str);
                status_item(ui, "🔄", &format!("{}°", match self.view_state.rotation {
                    Rotation::R0 => 0, Rotation::R90 => 90,
                    Rotation::R180 => 180, Rotation::R270 => 270,
                }));
            }

            if let Some(err) = &self.status {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(err)
                            .color(Color32::from_rgb(255, 96, 96))
                            .size(11.0),
                    );
                });
            } else {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new("← → 탐색  |  +/− 줌  |  I 정보  |  G 갤러리  |  T 격자  |  L/R 회전")
                            .size(10.0)
                            .color(Color32::from_rgb(85, 85, 95)),
                    );
                });
            }
        });
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper functions
// ─────────────────────────────────────────────────────────────────────────────

/// Draw checkerboard pattern within `rect`.
fn draw_checkerboard(painter: &Painter, rect: Rect) {
    const CELL: f32 = 12.0;
    let cols = ((rect.width()  / CELL) as usize) + 2;
    let rows = ((rect.height() / CELL) as usize) + 2;
    let c1 = Color32::from_rgb(58, 58, 58);
    let c2 = Color32::from_rgb(44, 44, 44);

    for row in 0..rows {
        for col in 0..cols {
            let x = rect.left() + col as f32 * CELL;
            let y = rect.top()  + row as f32 * CELL;
            let cell = Rect::from_min_size(
                Pos2::new(x, y),
                Vec2::splat(CELL),
            ).intersect(rect);
            if cell.area() < 0.01 { continue; }
            let color = if (row + col) % 2 == 0 { c1 } else { c2 };
            painter.rect_filled(cell, 0.0, color);
        }
    }
}

/// Calculate the scale to fit (w, h) into `available` rect.
fn fit_scale(available: Rect, w: f32, h: f32) -> f32 {
    if w == 0.0 || h == 0.0 { return 1.0; }
    let sx = available.width()  / w;
    let sy = available.height() / h;
    sx.min(sy).min(1.0) // never upscale beyond 100% in fit mode
}

/// Draw a nav arrow button; returns true if clicked.
fn nav_arrow(ui: &mut egui::Ui, rect: Rect, label: &str) -> bool {
    let response = ui.allocate_rect(rect, egui::Sense::click());
    let bg = if response.hovered() {
        Color32::from_rgba_unmultiplied(255, 255, 255, 30)
    } else {
        Color32::from_rgba_unmultiplied(255, 255, 255, 12)
    };
    ui.painter().rect_filled(rect, 8.0, bg);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(22.0),
        Color32::from_rgb(152, 152, 168),
    );
    response.clicked()
}

/// Toolbar separator line.
fn separator(ui: &mut egui::Ui) {
    ui.add(egui::Separator::default().vertical().spacing(8.0));
}


/// Status bar item.
fn status_item(ui: &mut egui::Ui, icon: &str, val: &str) {
    ui.label(
        egui::RichText::new(format!("{icon} {val}"))
            .size(11.0)
            .color(Color32::from_rgb(85, 85, 95)),
    );
    ui.add_space(8.0);
}


// ─────────────────────────────────────────────────────────────────────────────
// Config persistence
// ─────────────────────────────────────────────────────────────────────────────

fn config_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("rustview").join("config.toml"))
}

fn load_config() -> AppConfig {
    let Some(path) = config_path() else { return AppConfig::default(); };
    let Ok(text) = std::fs::read_to_string(&path) else { return AppConfig::default(); };
    toml::from_str(&text).unwrap_or_default()
}

fn save_config(config: &AppConfig) {
    let Some(path) = config_path() else { return; };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = toml::to_string(config) {
        let _ = std::fs::write(path, text);
    }
}
