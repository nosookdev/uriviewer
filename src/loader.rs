// loader.rs — Image loading and EXIF extraction

use std::path::Path;
use egui::Context;
use image::DynamicImage;

use crate::types::{ExifData, LoadedImage};

// ── Format detection ──────────────────────────────────────────────────────

pub fn format_from_path(path: &Path) -> String {
    match path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .as_deref()
    {
        Some("jpg") | Some("jpeg") => "JPEG",
        Some("png")                => "PNG",
        Some("gif")                => "GIF",
        Some("bmp")                => "BMP",
        Some("webp")               => "WebP",
        Some("tif") | Some("tiff") => "TIFF",
        Some("ico")                => "ICO",
        Some("svg")                => "SVG",
        _                          => "Unknown",
    }
    .to_string()
}

// ── EXIF ──────────────────────────────────────────────────────────────────

pub fn read_exif(path: &Path) -> Option<ExifData> {
    let file = std::fs::File::open(path).ok()?;
    let mut buf = std::io::BufReader::new(file);
    let reader = exif::Reader::new();
    let exif = reader.read_from_container(&mut buf).ok()?;

    let get = |tag: exif::Tag| -> Option<String> {
        exif.get_field(tag, exif::In::PRIMARY)
            .map(|f| f.display_value().with_unit(&exif).to_string())
    };

    Some(ExifData {
        camera_make:  get(exif::Tag::Make),
        camera_model: get(exif::Tag::Model),
        lens:         get(exif::Tag::LensModel),
        exposure:     get(exif::Tag::ExposureTime),
        f_number:     get(exif::Tag::FNumber),
        iso:          get(exif::Tag::PhotographicSensitivity),
        focal_length: get(exif::Tag::FocalLength),
        date_taken:   get(exif::Tag::DateTimeOriginal),
    })
}

// ── Image loading ─────────────────────────────────────────────────────────

/// Load an image from disk and upload it as an egui texture.
pub fn load_image(ctx: &Context, path: &Path) -> Result<LoadedImage, String> {
    let file_size = std::fs::metadata(path)
        .map(|m| m.len())
        .unwrap_or(0);

    let format = format_from_path(path);

    let dyn_img: DynamicImage = image::open(path)
        .map_err(|e| format!("이미지를 열 수 없습니다: {e}"))?;

    let orig_w = dyn_img.width();
    let orig_h = dyn_img.height();

    let rgba = dyn_img.to_rgba8();
    let pixels = rgba.as_raw();

    let color_image = egui::ColorImage::from_rgba_unmultiplied(
        [orig_w as usize, orig_h as usize],
        pixels,
    );

    let name = path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let texture = ctx.load_texture(
        &name,
        color_image,
        egui::TextureOptions::LINEAR,
    );

    let exif = read_exif(path);

    Ok(LoadedImage {
        path: path.to_path_buf(),
        texture,
        orig_w,
        orig_h,
        file_size,
        format,
        exif,
    })
}

// ── Thumbnail loading ─────────────────────────────────────────────────────

const THUMB_PX: u32 = 256;

pub fn load_thumbnail(ctx: &Context, path: &Path) -> Result<egui::TextureHandle, String> {
    let dyn_img = image::open(path)
        .map_err(|e| e.to_string())?;

    // Fit into THUMB_PX × THUMB_PX box
    let thumb = dyn_img.thumbnail(THUMB_PX, THUMB_PX);
    let rgba  = thumb.to_rgba8();
    let (tw, th) = (thumb.width() as usize, thumb.height() as usize);

    let color_image = egui::ColorImage::from_rgba_unmultiplied(
        [tw, th],
        rgba.as_raw(),
    );

    let name = format!(
        "thumb:{}",
        path.file_name().unwrap_or_default().to_string_lossy()
    );

    Ok(ctx.load_texture(name, color_image, egui::TextureOptions::LINEAR))
}

// ── Human-readable file size ──────────────────────────────────────────────

pub fn human_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}
