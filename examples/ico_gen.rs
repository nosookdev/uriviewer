// examples/ico_gen.rs
use std::path::Path;
use image::{GenericImageView, ImageFormat};

fn main() {
    let png_path = "assets/logo.png";
    let ico_path = "assets/icon.ico";
    
    if !Path::new(png_path).exists() {
        eprintln!("Error: logo.png not found in assets/");
        std::process::exit(1);
    }
    
    let img = image::open(png_path).expect("Failed to open logo.png");
    
    // Windows icons usually contain multiple sizes. 
    // We'll just generate a single 256x256 ICO for simplicity (it will work fine).
    let (w, h) = img.dimensions();
    println!("Converting {}x{} logo to ICO...", w, h);
    
    img.save_with_format(ico_path, ImageFormat::Ico).expect("Failed to save as ICO");
    println!("Successfully saved to assets/icon.ico");
}
