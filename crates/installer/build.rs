#[cfg(windows)]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("../app/ui/icon.ico");
    let sdk_bin = r"C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64";
    if std::path::Path::new(sdk_bin).join("rc.exe").exists() {
        res.set_toolkit_path(sdk_bin);
    }
    if let Err(e) = res.compile() {
        eprintln!("cargo:warning=Failed to compile Windows resource: {}", e);
    }

    // Ensure assets directory and fallback binary exist so cargo check / test succeed
    let assets_dir = std::path::Path::new("assets");
    if !assets_dir.exists() {
        let _ = std::fs::create_dir_all(assets_dir);
    }
    let payload_path = assets_dir.join("evergreen-browser.exe");
    if !payload_path.exists() {
        let _ = std::fs::write(&payload_path, b"");
    }
}

#[cfg(not(windows))]
fn main() {}
