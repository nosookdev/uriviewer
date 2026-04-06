// types.rs — Shared data types for RustView

use egui::TextureHandle;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ── View Mode ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    Viewer,
    Gallery,
}

// ── Scale Mode ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ScaleMode {
    Fit,      // 윈도우에 맞춤 (Aspect Fit)
    Fill,     // 윈도우에 꽉 차게 (Aspect Fill)
    Original, // 원본 크기 (1:1)
}

// ── Theme Mode ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AppTheme {
    Dark,
    Light,
}

// ── Rotation ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Rotation {
    R0,
    R90,
    R180,
    R270,
}

impl Rotation {
    pub fn cw(self) -> Self {
        match self {
            Self::R0 => Self::R90,
            Self::R90 => Self::R180,
            Self::R180 => Self::R270,
            Self::R270 => Self::R0,
        }
    }
    pub fn ccw(self) -> Self {
        match self {
            Self::R0 => Self::R270,
            Self::R90 => Self::R0,
            Self::R180 => Self::R90,
            Self::R270 => Self::R180,
        }
    }
    pub fn is_transposed(self) -> bool {
        matches!(self, Self::R90 | Self::R270)
    }
}

// ── EXIF Data ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct ExifData {
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub lens: Option<String>,
    pub exposure: Option<String>,
    pub f_number: Option<String>,
    pub iso: Option<String>,
    pub focal_length: Option<String>,
    pub date_taken: Option<String>,
    pub software: Option<String>,
    pub artist: Option<String>,
    pub orientation: Option<String>,
    pub orientation_num: Option<u16>,
    pub gps: Option<String>,
}

// ── File System Metadata (General/Details) ──────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct FileSystemMetadata {
    // General Tab
    pub name: String,
    pub file_type: String,
    pub location: String,
    pub size: u64,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub accessed: Option<String>,
    pub attributes: Option<String>,
    
    // Details Tab (More specific for images)
    pub width: u32,
    pub height: u32,
    pub bit_depth: Option<String>,
    pub color_space: Option<String>,
    pub dpi: Option<String>,
}

// ── Animation ─────────────────────────────────────────────────────────────

pub struct AnimationFrame {
    pub texture: TextureHandle,
    pub delay_ms: u32,
}

pub struct AnimatedImage {
    pub frames: Vec<AnimationFrame>,
    pub total_duration_ms: u32,
}

// ── Loaded Image ──────────────────────────────────────────────────────────

pub struct LoadedImage {
    pub path: PathBuf,
    pub texture: TextureHandle,
    pub orig_w: u32,
    pub orig_h: u32,
    pub file_size: u64,
    pub format: String,
    pub exif: Option<ExifData>,
    pub fs_metadata: FileSystemMetadata,
    pub animation: Option<AnimatedImage>,
}

impl LoadedImage {
    pub fn display_size(&self, rotation: Rotation) -> (u32, u32) {
        if rotation.is_transposed() { (self.orig_h, self.orig_w) } else { (self.orig_w, self.orig_h) }
    }
}

// ── View State ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SelectionState {
    pub start: egui::Pos2,
    pub end: egui::Pos2,
    pub active: bool,
}

pub struct ViewState {
    pub scale: f32,
    pub offset: egui::Vec2,
    pub rotation: Rotation,
    pub scale_mode: ScaleMode,
    pub checker: bool,
    pub anim_playing: bool,
    pub anim_frame: usize,
    pub anim_time_ms: u32,
    pub anim_loop: bool,
    pub selection_mode: bool,
    pub selection: Option<SelectionState>,
    pub theme_anim_start: Option<f64>,
    pub lock_scale: bool,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            scale: 1.0,
            offset: egui::Vec2::ZERO,
            rotation: Rotation::R0,
            scale_mode: ScaleMode::Fill, // 기본값: 꽉차게
            checker: true,
            anim_playing: true,
            anim_frame: 0,
            anim_time_ms: 0,
            anim_loop: true,
            selection_mode: false,
            selection: None,
            theme_anim_start: None,
            lock_scale: false,
        }
    }
}

impl ViewState {
    pub fn reset(&mut self, keep_scale: bool) {
        let saved_scale = self.scale;
        let saved_mode = self.scale_mode;
        
        self.scale = 1.0;
        self.offset = egui::Vec2::ZERO;
        self.rotation = Rotation::R0;
        self.anim_frame = 0;
        self.anim_time_ms = 0;
        self.selection_mode = false;
        self.selection = None;
        self.theme_anim_start = None;
        
        if keep_scale {
            self.scale = saved_scale;
            self.scale_mode = saved_mode;
        } else {
            self.scale_mode = ScaleMode::Fill; // 기본값
        }
    }
}

// ── Gallery ───────────────────────────────────────────────────────────────

pub enum ThumbState {
    Loading,
    Loaded(TextureHandle),
    Failed,
}

pub struct Gallery {
    pub folder: PathBuf,
    pub selected: usize,
    pub thumbs: HashMap<PathBuf, ThumbState>,
}

impl Gallery {
    pub fn new(folder: PathBuf, selected: usize) -> Self {
        Self {
            folder,
            selected,
            thumbs: HashMap::new(),
        }
    }
}

// ── App Config ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub info_open: bool,
    pub checker: bool,
    pub thumb_size: f32,
    pub last_directory: Option<PathBuf>,
    pub capture_hotkey: Option<(u32, u32)>,      // (modifiers, key_code)
    pub color_picker_hotkey: Option<(u32, u32)>, // (modifiers, key_code)
    pub use_shell_ext: bool,
    pub associated_extensions: std::collections::HashSet<String>,
    pub show_assoc_notification: bool,
    pub theme: AppTheme,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            info_open: true, // 기본값: 정보 패널 항상 열기
            checker: true,
            thumb_size: 140.0,
            last_directory: None,
            capture_hotkey: Some((6, 0x53)),      
            color_picker_hotkey: Some((6, 0x43)), 
            use_shell_ext: true, // 기본값: 탐색기 메뉴 사용
            associated_extensions: std::collections::HashSet::new(), // 초기엔 비어있음 (시스템 설정에 따라 다름)
            show_assoc_notification: false,
            theme: AppTheme::Dark,
        }
    }
}
