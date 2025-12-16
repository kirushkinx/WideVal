fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        winres::WindowsResource::new()
            .set_icon("icon.ico")
            .compile()
            .unwrap_or_else(|_| {
                println!("cargo:warning=Could not embed icon");
            });
    }
}