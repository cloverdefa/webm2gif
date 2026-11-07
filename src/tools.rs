//! 外部工具（ImageMagick / FFmpeg）的定位：內嵌 → 程式旁 tools/ → PATH。

use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};
use walkdir::WalkDir;

#[cfg(embedded_tools)]
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded_tools.rs"));
}

#[cfg(embedded_tools)]
fn extract_embedded_tools(root: &Path) -> Option<()> {
    for (relative, bytes) in embedded::EMBEDDED_TOOLS {
        let destination = root.join(*relative);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).ok()?;
        }
        std::fs::write(destination, *bytes).ok()?;
    }
    Some(())
}

fn embedded_tools_dir() -> Option<PathBuf> {
    static TOOLS: OnceLock<Option<PathBuf>> = OnceLock::new();
    TOOLS
        .get_or_init(|| {
            #[cfg(embedded_tools)]
            {
                // 以 APP_VERSION 命名，更新版本後會重新解壓新的工具。
                let root =
                    std::env::temp_dir().join(format!("webm2gif-tools-{}", env!("APP_VERSION")));
                let magick = root.join("ImageMagick/magick.exe");
                let ffmpeg = root.join("ffmpeg/ffmpeg.exe");
                if magick.is_file() && ffmpeg.is_file() {
                    return Some(root);
                }
                if std::fs::create_dir_all(&root).is_err() {
                    return None;
                }
                extract_embedded_tools(&root)?;
                if magick.is_file() && ffmpeg.is_file() {
                    Some(root)
                } else {
                    None
                }
            }
            #[cfg(not(embedded_tools))]
            {
                None
            }
        })
        .clone()
}

pub fn magick() -> Option<PathBuf> {
    if let Some(root) = embedded_tools_dir() {
        let path = root.join("ImageMagick/magick.exe");
        if path.is_file() {
            return Some(path);
        }
    }
    if let Some(exe_dir) = std::env::current_exe()
        .ok()?
        .parent()
        .map(Path::to_path_buf)
    {
        for root in [
            exe_dir.join("tools/ImageMagick"),
            exe_dir.join("ImageMagick-7.1.2-29-portable-Q16-x64"),
        ] {
            if let Some(path) = WalkDir::new(root)
                .into_iter()
                .filter_map(Result::ok)
                .find(|e| {
                    e.file_type().is_file()
                        && e.file_name().to_string_lossy().eq_ignore_ascii_case(
                            if cfg!(windows) {
                                "magick.exe"
                            } else {
                                "magick"
                            },
                        )
                })
                .map(|e| e.path().to_path_buf())
            {
                return Some(path);
            }
        }
    }
    if let Ok(path) = which("magick") {
        return Some(path);
    }
    // ImageMagick 6, still commonly packaged by Ubuntu releases used in WSL,
    // exposes the converter as `convert` instead of `magick`.
    if let Ok(path) = which("convert") {
        return Some(path);
    }
    None
}

pub fn ffmpeg() -> Option<PathBuf> {
    if let Some(root) = embedded_tools_dir() {
        let path = root.join("ffmpeg/ffmpeg.exe");
        if path.is_file() {
            return Some(path);
        }
    }
    if let Some(exe_dir) = std::env::current_exe().ok()?.parent() {
        let bundled = exe_dir.join("tools/ffmpeg").join(if cfg!(windows) {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        });
        if bundled.is_file() {
            return Some(bundled);
        }
        let bundled_bin = exe_dir.join("tools/ffmpeg/bin").join(if cfg!(windows) {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        });
        if bundled_bin.is_file() {
            return Some(bundled_bin);
        }
    }
    which("ffmpeg").ok()
}

fn which(name: &str) -> Result<PathBuf, ()> {
    let paths: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|value| std::env::split_paths(&value).collect())
        .unwrap_or_default();
    for p in paths {
        let candidate = p.join(name);
        if candidate.is_file() {
            return Ok(candidate);
        }
        #[cfg(windows)]
        {
            let exe = p.join(format!("{name}.exe"));
            if exe.is_file() {
                return Ok(exe);
            }
        }
    }
    Err(())
}
