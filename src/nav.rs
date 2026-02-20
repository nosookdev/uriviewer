// nav.rs — Folder-based image navigation

use std::path::{Path, PathBuf};

/// Extensions recognised as image files
const IMAGE_EXTS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "webp", "tif", "tiff", "ico",
];

pub fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Return all image files in the same folder as `current`, sorted by name.
pub fn images_in_folder(current: &Path) -> Vec<PathBuf> {
    let dir = match current.parent() {
        Some(d) => d,
        None    => return vec![],
    };

    let mut images: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_image(p))
        .collect();

    images.sort_by(|a, b| {
        a.file_name().cmp(&b.file_name())
    });

    images
}

/// Index of `current` in the sorted folder list.
pub fn current_index(images: &[PathBuf], current: &Path) -> usize {
    images.iter().position(|p| p == current).unwrap_or(0)
}
