//! 與作業系統相關的小工具。

use std::{ffi::OsStr, process::Command};

#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// 以系統預設程式開啟資料夾或網址。
pub fn open(target: impl AsRef<OsStr>) {
    let target = target.as_ref();
    #[cfg(target_os = "macos")]
    let _ = Command::new("open").arg(target).spawn();
    #[cfg(target_os = "windows")]
    let _ = Command::new("explorer").arg(target).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = Command::new("xdg-open").arg(target).spawn();
}
