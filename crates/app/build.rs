#[cfg(windows)]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("ui/icon.ico");
    let sdk_bin = r"C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64";
    if std::path::Path::new(sdk_bin).join("rc.exe").exists() {
        res.set_toolkit_path(sdk_bin);
    }
    if let Err(e) = res.compile() {
        eprintln!("cargo:warning=Failed to compile Windows resource: {}", e);
    }
}

#[cfg(not(windows))]
fn main() {}
