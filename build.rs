fn main() {
    println!("cargo:rerun-if-changed=assets/app-icon.ico");

    #[cfg(windows)]
    {
        let mut resources = winres::WindowsResource::new();
        resources.set_icon("assets/app-icon.ico");
        resources
            .compile()
            .expect("failed to embed the application icon");
    }
}
