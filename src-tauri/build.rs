fn main() {
    tauri_build::build();
    
    // Set environment variable to trigger post-processing
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-env=MARK_US_DOWN_BUILD=1");
    }
}