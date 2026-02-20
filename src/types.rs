// types.rs — Shared data types for RustView

use std::path::PathBuf;
use std::collections::HashMap;
use egui::TextureHandle;
use serde::{Deserialize, Serialize};

// ── View Mode ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    Viewer,
    Gallery,
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
            Self::R0   => Self::R90,
            Self::R90  => Self::R180,
            Self::R180 => Self::R270,
            Self::R270 => Self::R0,
        }
    }
    pub fn ccw(self) -> Self {
        match self {
            Self::R0   => Self::R270,
            Self::R90  => Self::R0,
            Self::R180 => Self::R90,
            Self::R270 => Self::R180,
        }
    }
    pub fn to_radians(self) -> f32 {
        match self {
            Self::R0   => 0.0,
            Self::R90  => std::f32::consts::FRAC_PI_2,
            Self::R180 => std::f32::consts::PI,
            Self::R270 => 3.0 * std::f32::consts::FRAC_PI_2,
        }
    }
    /// True when 90° or 270°: width and height are swapped
    pub fn is_transposed(self) -> bool {
        matches!(self, Self::R90 | Self::R270)
    }
}

// ── EXIF Data ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct ExifData {
    pub camera_make:  Option<String>,
    pub camera_model: Option<String>,
    pub lens:         Option<String>,
    pub exposure:     Option<String>,
    pub f_number:     Option<String>,
    pub iso:          Option<String>,
    pub focal_length: Option<String>,
    pub date_taken:   Option<String>,
}

// ── Loaded Image ──────────────────────────────────────────────────────────

pub struct LoadedImage {
    pub path:      PathBuf,
    pub texture:   TextureHandle,
    /// Original pixel dimensions (before rotation)
    pub orig_w:    u32,
    pub orig_h:    u32,
    pub file_size: u64,
    pub format:    String,
    pub exif:      Option<ExifData>,
}

impl LoadedImage {
    /// Display dimensions after applying rotation
    pub fn display_size(&self, rotation: Rotation) -> (u32, u32) {
        if rotation.is_transposed() {
            (self.orig_h, self.orig_w)
        } else {
            (self.orig_w, self.orig_h)
        }
    }
}

// ── View State ────────────────────────────────────────────────────────────

pub struct ViewState {
    /// Zoom multiplier: 1.0 = 100% (original pixels).
    /// When `fit_mode` is true this is ignored for rendering
    /// and re-calculated to fit the panel.
    pub scale:    f32,
    /// Pan offset in screen pixels from the panel centre
    pub offset:   egui::Vec2,
    pub rotation: Rotation,
    /// If true, image is fitted to the panel on next frame
    pub fit_mode: bool,
    /// Show checkerboard behind transparent images
    pub checker:  bool,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            scale:    1.0,
            offset:   egui::Vec2::ZERO,
            rotation: Rotation::R0,
            fit_mode: true,
            checker:  true,
        }
    }
}

impl ViewState {
    pub fn reset(&mut self) {
        self.scale    = 1.0;
        self.offset   = egui::Vec2::ZERO;
        self.rotation = Rotation::R0;
        self.fit_mode = true;
    }
}

// ── Thumbnail ─────────────────────────────────────────────────────────────

pub enum ThumbState {
    Loaded(TextureHandle),
    Failed,
}

// ── Gallery ───────────────────────────────────────────────────────────────

pub struct Gallery {
    pub folder:     PathBuf,
    pub selected:   usize,
    pub thumbs:     HashMap<PathBuf, ThumbState>,
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
    pub info_open:      bool,
    pub checker:        bool,
    pub thumb_size:     f32,
    pub last_directory: Option<PathBuf>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            info_open:      true,
            checker:        true,
            thumb_size:     140.0,
            last_directory: None,
        }
    }
}
