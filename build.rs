// build.rs
fn main() {
    #[cfg(windows)]
    {
        if std::path::Path::new("assets/icon.ico").exists() {
            let mut res = winres::WindowsResource::new();
            res.set_icon("assets/icon.ico");
            if let Err(e) = res.compile() {
                eprintln!("Error compiling resources: {}", e);
                std::process::exit(1);
            }
        }
    }
}
