fn main() {
    tauri_build::build();

    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("icons/icon.ico");
        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=icon gömme hatası: {e}");
        }
    }
}
