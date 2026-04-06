// nav.rs — Folder-based image navigation

use std::path::{Path, PathBuf};

/// Extensions recognised as image files
pub fn is_image(path: &Path) -> bool {
    let ext_list = [
        "jpg", "jpeg", "jpe", "png", "gif", "bmp", "webp", "tiff", "tif", "ico",
        "svg", "ani", "cal", "emf", "fax", "hdp", "mac", "pbm", "pcd", "pcx",
        "pgm", "ppm", "psd", "ras", "tga", "wmf",
        "cgm", "dwg", "dwf", "dxf", "iges", "obj", "plt", "step", "stl", "3ds"
    ];
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        let e = ext.to_lowercase();
        return ext_list.contains(&e.as_str());
    }
    false
}

/// Return all image files in the same folder as `current`, sorted by name.
pub fn images_in_folder(current: &Path) -> Vec<PathBuf> {
    let dir = if current.is_dir() {
        current
    } else {
        match current.parent() {
            Some(d) => d,
            None    => return vec![],
        }
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
