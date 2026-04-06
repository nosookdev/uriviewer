// capture.rs — Screen capture engine using xcap

use xcap::Monitor;
use image::RgbaImage;

pub struct CapturedScreen {
    pub monitor_name: String,
    pub image: RgbaImage,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub fn capture_all_screens() -> Vec<CapturedScreen> {
    let monitors = Monitor::all().unwrap_or_default();
    let mut screens = Vec::new();

    for monitor in monitors {
        if let Ok(img) = monitor.capture_image() {
            screens.push(CapturedScreen {
                monitor_name: monitor.name().to_string(),
                image: img,
                x: monitor.x(),
                y: monitor.y(),
                width: monitor.width(),
                height: monitor.height(),
            });
        }
    }
    screens
}
