// loader.rs — Image loading and EXIF extraction

use egui::{ColorImage, Context};
use image::{AnimationDecoder, DynamicImage};
use std::io::{BufReader, Cursor, Read};
use std::path::Path;
use chrono::{DateTime, Local};
use zip::ZipArchive;

use crate::types::{AnimatedImage, AnimationFrame, ExifData, LoadedImage};

pub fn load_from_zip(ctx: &egui::Context, archive_path: &Path, file_name: &str) -> Result<LoadedImage, String> {
    let file = std::fs::File::open(archive_path).map_err(|e| e.to_string())?;
    let mut zip = ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut inner_file = zip.by_name(file_name).map_err(|e| e.to_string())?;
    
    let mut buffer = Vec::new();
    inner_file.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
    
    let format = format_from_path(Path::new(file_name));
    let img = if format == "SVG" {
        _load_svg_from_data(ctx, &buffer, file_name, archive_path, buffer.len() as u64)?
    } else {
        let dynamic_img = image::load_from_memory(&buffer).map_err(|e| e.to_string())?;
        let rgba = dynamic_img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let pixels = rgba.as_raw();
        let color_img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], pixels);
        let tex = ctx.load_texture(file_name, color_img, egui::TextureOptions::LINEAR);
        
        LoadedImage {
            path: archive_path.to_path_buf(),
            texture: tex,
            orig_w: w,
            orig_h: h,
            file_size: buffer.len() as u64,
            format,
            exif: None,
            fs_metadata: crate::types::FileSystemMetadata {
                name: file_name.to_string(),
                location: archive_path.to_string_lossy().to_string(),
                size: buffer.len() as u64,
                width: 0, // Zip 내부 정보는 복잡하므로 최소화
                height: 0,
                file_type: "압축 내 파일".to_string(),
                ..Default::default()
            },
            animation: None,
        }
    };
    Ok(img)
}

// ── Format detection ──────────────────────────────────────────────────────

pub fn format_from_path(path: &Path) -> String {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .as_deref()
    {
        Some("jpg") | Some("jpeg") => "JPEG",
        Some("png") => "PNG",
        Some("gif") => "GIF",
        Some("bmp") => "BMP",
        Some("webp") => "WebP",
        Some("tif") | Some("tiff") => "TIFF",
        Some("ico") => "ICO",
        Some("svg") => "SVG",
        _ => "Unknown",
    }
    .to_string()
}

// ── EXIF ──────────────────────────────────────────────────────────────────

pub fn read_exif(path: &Path) -> Option<ExifData> {
    let file = std::fs::File::open(path).ok()?;
    let mut buf = std::io::BufReader::new(file);
    let reader = exif::Reader::new();
    let exif = reader.read_from_container(&mut buf)
        .or_else(|_| {
            let mut data = Vec::new();
            let mut file = std::fs::File::open(path).map_err(|e| exif::Error::Io(e))?;
            file.read_to_end(&mut data).map_err(|e| exif::Error::Io(e))?;
            reader.read_raw(data)
        }).ok()?;

    let get = |tag: exif::Tag| -> Option<String> {
        exif.get_field(tag, exif::In::PRIMARY)
            .map(|f| f.display_value().with_unit(&exif).to_string())
    };

    let gps = if let (Some(lat), Some(lon)) = (
        exif.get_field(exif::Tag::GPSLatitude, exif::In::PRIMARY),
        exif.get_field(exif::Tag::GPSLongitude, exif::In::PRIMARY),
    ) {
        Some(format!("{}, {}", lat.display_value(), lon.display_value()))
    } else {
        None
    };

    Some(ExifData {
        camera_make: get(exif::Tag::Make),
        camera_model: get(exif::Tag::Model),
        lens: get(exif::Tag::LensModel),
        exposure: get(exif::Tag::ExposureTime),
        f_number: get(exif::Tag::FNumber),
        iso: get(exif::Tag::PhotographicSensitivity),
        focal_length: get(exif::Tag::FocalLength),
        date_taken: get(exif::Tag::DateTimeOriginal),
        software: get(exif::Tag::Software),
        artist: get(exif::Tag::Artist),
        orientation: get(exif::Tag::Orientation),
        orientation_num: exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)
            .and_then(|f| match f.value {
                exif::Value::Short(ref v) if !v.is_empty() => Some(v[0]),
                exif::Value::Long(ref v) if !v.is_empty() => Some(v[0] as u16),
                _ => None,
            }),
        gps,
    })
}

pub fn get_fs_metadata(path: &Path, w: u32, h: u32) -> crate::types::FileSystemMetadata {
    let metadata = std::fs::metadata(path);
    let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
    let location = path.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let file_type = format!("{} 파일", path.extension().unwrap_or_default().to_string_lossy().to_uppercase());
    
    let mut fs_meta = crate::types::FileSystemMetadata {
        name,
        file_type,
        location,
        size: metadata.as_ref().map(|m| m.len()).unwrap_or(0),
        width: w,
        height: h,
        ..Default::default()
    };

    if let Ok(m) = metadata {
        let fmt_time = |t: std::time::SystemTime| {
            let datetime: DateTime<Local> = t.into();
            datetime.format("%Y-%m-%d %p %I:%M:%S").to_string()
        };

        fs_meta.created = m.created().ok().map(fmt_time);
        fs_meta.modified = m.modified().ok().map(fmt_time);
        fs_meta.accessed = m.accessed().ok().map(fmt_time);

        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            let attr = m.file_attributes();
            let mut attrs = Vec::new();
            if attr & 1 != 0 { attrs.push("읽기 전용"); }
            if attr & 2 != 0 { attrs.push("숨김"); }
            if attr & 4 != 0 { attrs.push("시스템"); }
            if attr & 32 != 0 { attrs.push("보관"); }
            if !attrs.is_empty() {
                fs_meta.attributes = Some(attrs.join(", "));
            }
        }
    }

    fs_meta
}

// ── Image loading ─────────────────────────────────────────────────────────

/// Load an image from disk and upload it as an egui texture.
pub fn load_image(ctx: &Context, path: &Path) -> Result<LoadedImage, String> {
    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    let format = format_from_path(path);
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let mut animation = None;

    if format == "SVG" {
        return load_svg(ctx, path);
    }

    // Handle animated GIF
    if format == "GIF" {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let reader = BufReader::new(file);
        let decoder = image::codecs::gif::GifDecoder::new(reader).map_err(|e| e.to_string())?;

        let mut anim_frames = Vec::new();
        let mut total_duration_ms = 0;

        for (i, frame_result) in decoder.into_frames().enumerate() {
            let frame = frame_result.map_err(|e: image::ImageError| e.to_string())?;

            // Get delay BEFORE consuming frame
            let delay = frame.delay();
            let (num, den) = delay.numer_denom_ms();
            let delay_ms = if den == 0 { 100 } else { (num / den) as u32 };

            let buffer: image::RgbaImage = frame.into_buffer();
            let (w, h) = buffer.dimensions();

            let color_image =
                egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], buffer.as_raw());
            let tex_name = format!("{}:{}", name, i);
            let texture = ctx.load_texture(tex_name, color_image, egui::TextureOptions::LINEAR);

            anim_frames.push(AnimationFrame { texture, delay_ms });
            total_duration_ms += delay_ms;
        }

        if !anim_frames.is_empty() {
            animation = Some(AnimatedImage {
                frames: anim_frames,
                total_duration_ms,
            });
        }
    }

    let mut dyn_img: DynamicImage =
        image::open(path).map_err(|e| format!("이미지를 열 수 없습니다: {e}"))?;

    let orig_w = dyn_img.width();
    let orig_h = dyn_img.height();

    // egui texture size limit (typically 2048 or 4096 depending on GPU, 2048 is safe)
    const MAX_SIZE: u32 = 2048;
    if orig_w > MAX_SIZE || orig_h > MAX_SIZE {
        dyn_img = dyn_img.thumbnail(MAX_SIZE, MAX_SIZE);
    }

    let rgba = dyn_img.to_rgba8();
    let pixels = rgba.as_raw();

    let color_image = egui::ColorImage::from_rgba_unmultiplied(
        [dyn_img.width() as usize, dyn_img.height() as usize],
        pixels,
    );

    let texture = ctx.load_texture(&name, color_image, egui::TextureOptions::LINEAR);

    let mut fs_metadata = get_fs_metadata(path, orig_w, orig_h);
    
    // Extract bit depth and color space from DynamicImage
    let color_type = dyn_img.color();
    fs_metadata.bit_depth = Some(match color_type {
        image::ColorType::L8 | image::ColorType::Rgb8 | image::ColorType::Rgba8 => "8-bit".to_string(),
        image::ColorType::L16 | image::ColorType::Rgb16 | image::ColorType::Rgba16 => "16-bit".to_string(),
        image::ColorType::Rgb32F | image::ColorType::Rgba32F => "32-bit Float".to_string(),
        _ => format!("{:?}", color_type),
    });
    
    fs_metadata.color_space = Some(match color_type {
        image::ColorType::L8 | image::ColorType::L16 => "Grayscale".to_string(),
        image::ColorType::La8 | image::ColorType::La16 => "Grayscale with Alpha".to_string(),
        image::ColorType::Rgb8 | image::ColorType::Rgb16 | image::ColorType::Rgb32F => "RGB".to_string(),
        image::ColorType::Rgba8 | image::ColorType::Rgba16 | image::ColorType::Rgba32F => "RGBA".to_string(),
        _ => "Unknown".to_string(),
    });

    let exif = read_exif(path);

    Ok(LoadedImage {
        path: path.to_path_buf(),
        texture,
        orig_w,
        orig_h,
        file_size,
        format,
        exif,
        fs_metadata,
        animation,
    })
}

/// Load an SVG image from file path.
pub fn load_svg(ctx: &Context, path: &Path) -> Result<LoadedImage, String> {
    let svg_data = std::fs::read(path).map_err(|e| e.to_string())?;
    let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    _load_svg_from_data(ctx, &svg_data, &name, path, file_size)
}

fn _load_svg_from_data(ctx: &Context, svg_data: &[u8], name: &str, path: &Path, file_size: u64) -> Result<LoadedImage, String> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(svg_data, &opt).map_err(|e| e.to_string())?;
    
    let size = tree.size();
    let (mut w, mut h) = (size.width(), size.height());
    
    // Limit SVG rendering size for texture performance
    const MAX_SVG_SIZE: f32 = 2048.0;
    let mut zoom = 1.0;
    if w > MAX_SVG_SIZE || h > MAX_SVG_SIZE {
        zoom = (MAX_SVG_SIZE / w).min(MAX_SVG_SIZE / h);
        w *= zoom;
        h *= zoom;
    }

    let mut pixmap = resvg::tiny_skia::Pixmap::new(w as u32, h as u32)
        .ok_or_else(|| "Failed to create pixmap".to_string())?;
    
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(zoom, zoom), &mut pixmap.as_mut());

    let pixels = pixmap.data();
    let color_image = ColorImage::from_rgba_unmultiplied(
        [w as usize, h as usize],
        pixels,
    );
    
    let texture = ctx.load_texture(name, color_image, egui::TextureOptions::LINEAR);

    Ok(LoadedImage {
        path: path.to_path_buf(),
        texture,
        orig_w: w as u32,
        orig_h: h as u32,
        file_size,
        format: "SVG".to_string(),
        exif: None,
        fs_metadata: get_fs_metadata(path, w as u32, h as u32),
        animation: None,
    })
}

// ── Thumbnail loading ─────────────────────────────────────────────────────

const THUMB_PX: u32 = 256;

pub fn load_thumbnail(ctx: &Context, path: &Path) -> Result<egui::TextureHandle, String> {
    let color_image = decode_thumbnail(path)?;
    let name = format!("thumb:{}", path.file_name().unwrap_or_default().to_string_lossy());
    Ok(ctx.load_texture(name, color_image, egui::TextureOptions::LINEAR))
}

pub fn decode_thumbnail(path: &Path) -> Result<egui::ColorImage, String> {
    let format = format_from_path(path);
    if format == "SVG" {
        return decode_svg_thumbnail(path);
    }

    let dyn_img = image::open(path).map_err(|e| e.to_string())?;
    let thumb = dyn_img.thumbnail(THUMB_PX, THUMB_PX);
    let rgba = thumb.to_rgba8();
    let (tw, th) = (thumb.width() as usize, thumb.height() as usize);
    Ok(egui::ColorImage::from_rgba_unmultiplied([tw, th], rgba.as_raw()))
}

pub fn decode_svg_thumbnail(path: &Path) -> Result<egui::ColorImage, String> {
    let svg_data = std::fs::read(path).map_err(|e| e.to_string())?;
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(&svg_data, &opt).map_err(|e| e.to_string())?;
    let size = tree.size();
    let ratio = (THUMB_PX as f32 / size.width()).min(THUMB_PX as f32 / size.height());
    let (tw, th) = (size.width() * ratio, size.height() * ratio);
    let mut pixmap = resvg::tiny_skia::Pixmap::new(tw as u32, th as u32)
        .ok_or_else(|| "Failed to create thumb pixmap".to_string())?;
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(ratio, ratio), &mut pixmap.as_mut());
    Ok(ColorImage::from_rgba_unmultiplied([tw as usize, th as usize], pixmap.data()))
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
