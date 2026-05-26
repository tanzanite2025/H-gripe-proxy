fn main() {
    #[cfg(feature = "clippy")]
    {
        println!("cargo:warning=Skipping tauri_build during Clippy");
    }

    #[cfg(not(feature = "clippy"))]
    {
        // 告诉 Cargo 当图标文件改变时重新运行 build.rs
        // 这样可以避免使用缓存的旧图标
        println!("cargo:rerun-if-changed=icons/icon.ico");
        println!("cargo:rerun-if-changed=icons/32x32.png");
        println!("cargo:rerun-if-changed=icons/128x128.png");
        println!("cargo:rerun-if-changed=icons/128x128@2x.png");
        println!("cargo:rerun-if-changed=icons/icon.icns");
        
        tauri_build::build();
    }
}
