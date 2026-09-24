#[cfg(windows)]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("ui/icon.ico");
    if let Some(sdk_bin) = find_windows_sdk_bin() {
        res.set_toolkit_path(&sdk_bin);
    }
    if let Err(e) = res.compile() {
        eprintln!("cargo:warning=Failed to compile Windows resource: {}", e);
    }
}

#[cfg(windows)]
fn find_windows_sdk_bin() -> Option<String> {
    let prog_files = std::env::var("ProgramFiles(x86)")
        .or_else(|_| std::env::var("ProgramFiles"))
        .unwrap_or_else(|_| r"C:\Program Files (x86)".to_string());
    let base = std::path::PathBuf::from(prog_files).join(r"Windows Kits\10\bin");
    if let Ok(entries) = std::fs::read_dir(&base) {
        let mut sdk_dirs: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir() && e.path().join("x64").join("rc.exe").exists())
            .map(|e| e.path().join("x64"))
            .collect();
        sdk_dirs.sort();
        if let Some(highest) = sdk_dirs.pop() {
            return Some(highest.to_string_lossy().to_string());
        }
    }
    None
}

#[cfg(not(windows))]
fn main() {}
