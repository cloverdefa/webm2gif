fn main() {
    println!("cargo:rerun-if-changed=assets/app-icon.ico");
    println!("cargo:rerun-if-env-changed=WEBM2GIF_TOOLS_DIR");
    println!("cargo:rustc-check-cfg=cfg(embedded_tools)");

    if let Ok(source) = std::env::var("WEBM2GIF_TOOLS_DIR") {
        println!("cargo:rustc-cfg=embedded_tools");
        let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
        let source = std::path::PathBuf::from(source);
        let mut entries = Vec::new();
        collect_files(&source, &mut entries);
        let mut generated = String::from("pub static EMBEDDED_TOOLS: &[(&str, &[u8])] = &[\n");
        for path in entries {
            let relative = path.strip_prefix(&source).unwrap();
            println!("cargo:rerun-if-changed={}", path.display());
            let absolute = path.canonicalize().expect("failed to resolve tool file path");
            generated.push_str(&format!("({:?}, include_bytes!({:?})),\n", relative.to_string_lossy().replace('\\', "/"), absolute));
        }
        generated.push_str("];\n");
        std::fs::write(out.join("embedded_tools.rs"), generated).expect("failed to generate embedded tools source");
    }

    #[cfg(windows)]
    {
        let mut resources = winres::WindowsResource::new();
        resources.set_icon("assets/app-icon.ico");
        resources
            .compile()
            .expect("failed to embed the application icon");
    }
}

fn collect_files(dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("failed to read tools directory") {
        let path = entry.expect("failed to read tools entry").path();
        if path.is_dir() {
            collect_files(&path, files);
        } else if path.is_file() {
            files.push(path);
        }
    }
}
