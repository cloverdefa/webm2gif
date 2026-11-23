//! 從 GitHub Releases 檢查新版本，並下載、驗證、替換目前執行檔。
//!
//! 不使用 api.github.com（有速率限制），只用一般的 github.com 網址：
//!   * 最新版本：請求 `/releases/latest`，從 302 的 `Location` 讀出 tag。
//!   * 檔案存在與大小：對 `/releases/download/<tag>/<asset>` 送 HEAD。
//!   * 完整性：選用的 `<asset>.sha256` 側檔；沒有就略過，仍會檢查檔頭。
//!
//! 發佈資產命名（見 release.yml）：
//!   Windows x86_64 : `webm2gif.exe`
//!   Linux   x86_64 : `webm2gif-linux-x86_64.tar.gz`（內含 webm2gif-linux-x86_64/webm2gif）
//! 其他平台（macOS、aarch64 Pi）沒有對應檔案，只提示新版並引導到下載頁。

use eframe::egui;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

/// 目前版本。CI 以 WEBM2GIF_VERSION 傳入 tag，本機開發則為 Cargo.toml 的 version。
pub const CURRENT_VERSION: &str = env!("APP_VERSION");
pub const RELEASES_PAGE: &str = "https://github.com/cloverdefa/webm2gif/releases/latest";
const TAG_PAGE_PREFIX: &str = "https://github.com/cloverdefa/webm2gif/releases/tag/";
/// 只信任本 repo 的 release 下載網址，避免被導向其他來源。
const DOWNLOAD_PREFIX: &str = "https://github.com/cloverdefa/webm2gif/releases/download/";

/// 下載時每累積這麼多位元組回報一次進度。
const PROGRESS_STEP: u64 = 512 * 1024;
const READ_BUF_SIZE: usize = 64 * 1024;

// ───────────────────────── 型別 ─────────────────────────

#[derive(Clone, Debug)]
pub struct AssetInfo {
    pub name: String,
    pub url: String,
    /// HEAD 取不到大小時為 0；下載時會改用回應的 Content-Length。
    pub size: u64,
    pub sha256: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ReleaseInfo {
    pub version: String,
    pub page: String,
    /// 目前平台可自動安裝的檔案；None 代表只能手動下載。
    pub asset: Option<AssetInfo>,
}

#[derive(Clone, Debug)]
pub enum State {
    Idle,
    Checking,
    UpToDate,
    Available(ReleaseInfo),
    Installing { done: u64, total: u64 },
    /// 已安裝完成，內含新版本號；重新啟動後生效。
    Installed(String),
    Failed(String),
}

enum Msg {
    Checked(Result<Option<ReleaseInfo>, String>),
    Progress { done: u64, total: u64 },
    Installed(Result<(), String>),
}

// ───────────────────────── Updater ─────────────────────────

pub struct Updater {
    pub state: State,
    /// 啟動時記下執行檔路徑：Linux 替換後 current_exe() 會變成 "(deleted)"。
    exe: PathBuf,
    rx: Option<Receiver<Msg>>,
    /// 靜默檢查（啟動時）：失敗或已是最新都回到 Idle，不顯示任何訊息。
    silent: bool,
    pending_version: Option<String>,
}

impl Default for Updater {
    fn default() -> Self {
        Self::new()
    }
}

impl Updater {
    pub fn new() -> Self {
        Self {
            state: State::Idle,
            exe: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("webm2gif")),
            rx: None,
            silent: false,
            pending_version: None,
        }
    }

    pub fn busy(&self) -> bool {
        matches!(self.state, State::Checking | State::Installing { .. })
    }

    /// 在背景執行緒開啟 channel，worker 結束後會自動請求重繪。
    fn spawn_worker<F>(&mut self, ctx: &egui::Context, work: F)
    where
        F: FnOnce(mpsc::Sender<Msg>) + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        let ctx = ctx.clone();
        thread::spawn(move || {
            work(tx);
            ctx.request_repaint();
        });
    }

    pub fn check(&mut self, ctx: &egui::Context, silent: bool) {
        if self.busy() {
            return;
        }
        self.silent = silent;
        self.state = State::Checking;
        self.spawn_worker(ctx, |tx| {
            let _ = tx.send(Msg::Checked(fetch_latest()));
        });
    }

    /// 下載並安裝目前 `Available` 的版本。
    pub fn install(&mut self, ctx: &egui::Context) {
        let State::Available(info) = &self.state else {
            return;
        };
        let Some(asset) = info.asset.clone() else {
            return;
        };
        self.pending_version = Some(info.version.clone());
        self.state = State::Installing {
            done: 0,
            total: asset.size,
        };
        let progress_ctx = ctx.clone();
        self.spawn_worker(ctx, move |tx| {
            let progress_tx = tx.clone();
            let result = install_asset(&asset, |done, total| {
                let _ = progress_tx.send(Msg::Progress { done, total });
                progress_ctx.request_repaint();
            });
            let _ = tx.send(Msg::Installed(result));
        });
    }

    pub fn dismiss(&mut self) {
        if !self.busy() {
            self.state = State::Idle;
        }
    }

    /// 以更新後的執行檔啟動新程序；成功回傳 true，呼叫端再關閉目前視窗。
    pub fn restart(&mut self) -> bool {
        match Command::new(&self.exe).spawn() {
            Ok(_) => true,
            Err(e) => {
                self.state = State::Failed(format!("could not restart: {e}"));
                false
            }
        }
    }

    /// 每個 frame 呼叫一次，收取背景執行緒的結果。
    pub fn poll(&mut self) {
        let Some(rx) = &self.rx else {
            return;
        };
        let msgs: Vec<Msg> = rx.try_iter().collect();
        for msg in msgs {
            self.apply(msg);
        }
        if !self.busy() {
            self.rx = None;
        }
    }

    fn apply(&mut self, msg: Msg) {
        match msg {
            Msg::Checked(Ok(Some(info))) => self.state = State::Available(info),
            Msg::Checked(Ok(None)) => {
                self.state = if self.silent {
                    State::Idle
                } else {
                    State::UpToDate
                };
            }
            Msg::Checked(Err(e)) => {
                self.state = if self.silent {
                    State::Idle
                } else {
                    State::Failed(e)
                };
            }
            Msg::Progress { done, total } => {
                if matches!(self.state, State::Installing { .. }) {
                    self.state = State::Installing { done, total };
                }
            }
            Msg::Installed(Ok(())) => {
                self.state = State::Installed(self.pending_version.take().unwrap_or_default());
            }
            Msg::Installed(Err(e)) => self.state = State::Failed(e),
        }
    }
}

// ───────────────────────── 版本比較 ─────────────────────────

fn parse_version(s: &str) -> Option<(u64, u64, u64)> {
    let core = s.trim().trim_start_matches(['v', 'V']);
    let core = core.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    Some((major, minor, patch))
}

fn is_newer(remote: &str, local: &str) -> bool {
    match (parse_version(remote), parse_version(local)) {
        (Some(r), Some(l)) => r > l,
        _ => false,
    }
}

// ───────────────────────── 網路 ─────────────────────────

fn agent_builder() -> ureq::AgentBuilder {
    ureq::AgentBuilder::new()
        .user_agent(concat!("webm2gif/", env!("APP_VERSION")))
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(30))
}

fn agent() -> ureq::Agent {
    agent_builder().build()
}

/// 不跟隨轉址，用來從 /releases/latest 的 Location 讀出最新 tag。
fn no_redirect_agent() -> ureq::Agent {
    agent_builder().redirects(0).build()
}

fn describe(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(403 | 429, _) => "too many requests, try again later".into(),
        ureq::Error::Status(404, _) => "release not found".into(),
        ureq::Error::Status(code, _) => format!("HTTP {code}"),
        ureq::Error::Transport(t) => t.to_string(),
    }
}

fn platform_asset() -> Option<&'static str> {
    if cfg!(all(windows, target_arch = "x86_64")) {
        Some("webm2gif.exe")
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("webm2gif-linux-x86_64.tar.gz")
    } else {
        None
    }
}

/// 從 `.../releases/tag/v1.2.3` 取出 `v1.2.3`。
fn tag_from_location(location: &str) -> Option<String> {
    location
        .rsplit_once("/tag/")
        .map(|(_, tag)| tag.trim().to_owned())
        .filter(|tag| !tag.is_empty())
}

/// 請求 /releases/latest，從 302 的 Location 取得最新 tag。
fn latest_tag() -> Result<String, String> {
    let resp = match no_redirect_agent().get(RELEASES_PAGE).call() {
        Ok(r) => r,
        // 不同 ureq 版本可能把 3xx 當成錯誤回傳，兩種都處理。
        Err(ureq::Error::Status(code, r)) if (300..400).contains(&code) => r,
        Err(e) => return Err(describe(e)),
    };
    let location = resp.header("Location").ok_or("release not found")?;
    tag_from_location(location).ok_or_else(|| "cannot parse latest version".to_string())
}

/// 選用的 `<asset>.sha256` 側檔（格式 "<hex>  <name>"）；抓不到就回傳 None。
fn fetch_sha256(asset_url: &str) -> Option<String> {
    let text = agent()
        .get(&format!("{asset_url}.sha256"))
        .call()
        .ok()?
        .into_string()
        .ok()?;
    let hex = text.split_whitespace().next()?;
    (hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit()))
        .then(|| hex.to_ascii_lowercase())
}

/// 用 HEAD 確認檔案存在並取得大小。
fn probe_asset(tag: &str, name: &str) -> Option<AssetInfo> {
    let url = format!("{DOWNLOAD_PREFIX}{tag}/{name}");
    let head = agent().head(&url).call().ok()?;
    let size = head
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    let sha256 = fetch_sha256(&url);
    Some(AssetInfo {
        name: name.to_owned(),
        url,
        size,
        sha256,
    })
}

fn fetch_latest() -> Result<Option<ReleaseInfo>, String> {
    let tag = latest_tag()?;
    if !is_newer(&tag, CURRENT_VERSION) {
        return Ok(None);
    }
    let asset = platform_asset().and_then(|name| probe_asset(&tag, name));
    Ok(Some(ReleaseInfo {
        version: tag.trim_start_matches(['v', 'V']).to_owned(),
        page: format!("{TAG_PAGE_PREFIX}{tag}"),
        asset,
    }))
}

// ───────────────────────── 下載與安裝 ─────────────────────────

fn install_asset(asset: &AssetInfo, mut progress: impl FnMut(u64, u64)) -> Result<(), String> {
    if !asset.url.starts_with(DOWNLOAD_PREFIX) {
        return Err("unexpected download URL".into());
    }
    let dir = std::env::temp_dir().join(format!("webm2gif-update-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create temp dir: {e}"))?;
    let result = download_and_replace(asset, &dir, &mut progress);
    let _ = std::fs::remove_dir_all(&dir);
    result
}

fn download_and_replace(
    asset: &AssetInfo,
    dir: &Path,
    progress: &mut impl FnMut(u64, u64),
) -> Result<(), String> {
    let download = dir.join(&asset.name);
    let digest = download_file(asset, &download, progress)?;
    verify_checksum(asset.sha256.as_deref(), &digest)?;

    let new_exe = prepare_binary(&download, dir)?;
    check_magic(&new_exe)?;
    self_replace::self_replace(&new_exe).map_err(|e| format!("cannot replace executable: {e}"))
}

/// 下載到 `dest`，回傳內容的 SHA-256（小寫十六進位）。
fn download_file(
    asset: &AssetInfo,
    dest: &Path,
    progress: &mut impl FnMut(u64, u64),
) -> Result<String, String> {
    let response = agent().get(&asset.url).call().map_err(describe)?;
    let total = response
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(asset.size);
    let mut reader = response.into_reader();
    let write_err = |e: std::io::Error| format!("cannot write file: {e}");
    let mut file = File::create(dest).map_err(write_err)?;

    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; READ_BUF_SIZE];
    let (mut done, mut last_report) = (0u64, 0u64);
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("download interrupted: {e}"))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(write_err)?;
        hasher.update(&buf[..n]);
        done += n as u64;
        if done - last_report >= PROGRESS_STEP {
            last_report = done;
            progress(done, total);
        }
    }
    file.flush().map_err(write_err)?;
    drop(file);
    progress(done, total);

    if done == 0 || (total > 0 && done != total) {
        return Err("download incomplete".into());
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// 有提供預期雜湊時才驗證；沒有則略過（之後仍會做檔頭檢查）。
fn verify_checksum(expected: Option<&str>, actual: &str) -> Result<(), String> {
    match expected {
        Some(exp) if !actual.eq_ignore_ascii_case(exp) => {
            Err("checksum mismatch, update aborted".into())
        }
        _ => Ok(()),
    }
}

/// Linux：從 .tar.gz 取出 `webm2gif` 執行檔。
#[cfg(unix)]
fn prepare_binary(archive: &Path, dir: &Path) -> Result<PathBuf, String> {
    use std::os::unix::fs::PermissionsExt;
    let file = File::open(archive).map_err(|e| e.to_string())?;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
    for entry in tar.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path().map_err(|e| e.to_string())?.into_owned();
        let is_binary = entry.header().entry_type().is_file()
            && path.file_name().and_then(|n| n.to_str()) == Some("webm2gif");
        if is_binary {
            let out = dir.join("webm2gif.new");
            entry.unpack(&out).map_err(|e| e.to_string())?;
            std::fs::set_permissions(&out, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| e.to_string())?;
            return Ok(out);
        }
    }
    Err("executable not found in archive".into())
}

/// Windows：發佈檔本身就是 exe。
#[cfg(not(unix))]
fn prepare_binary(download: &Path, _dir: &Path) -> Result<PathBuf, String> {
    Ok(download.to_path_buf())
}

/// 簡單檢查檔頭，避免把錯誤頁面或損毀檔案換成執行檔。
fn check_magic(path: &Path) -> Result<(), String> {
    let mut head = [0u8; 4];
    File::open(path)
        .and_then(|mut f| f.read_exact(&mut head))
        .map_err(|e| format!("cannot read downloaded file: {e}"))?;
    let ok = if cfg!(windows) {
        &head[..2] == b"MZ"
    } else {
        &head == b"\x7fELF"
    };
    if ok {
        Ok(())
    } else {
        Err("downloaded file is not a valid executable".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compare() {
        assert!(is_newer("v0.2.0", "0.1.9"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(is_newer("v0.1.10", "0.1.9"));
        assert!(!is_newer("v0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
        assert!(!is_newer("garbage", "0.1.0"));
    }

    #[test]
    fn tag_parse() {
        assert_eq!(
            tag_from_location("https://github.com/cloverdefa/webm2gif/releases/tag/v0.2.0"),
            Some("v0.2.0".into())
        );
        assert_eq!(
            tag_from_location("https://github.com/cloverdefa/webm2gif/releases"),
            None
        );
    }

    #[test]
    fn checksum_check() {
        assert!(verify_checksum(None, "abc").is_ok());
        assert!(verify_checksum(Some("ABC"), "abc").is_ok());
        assert!(verify_checksum(Some("def"), "abc").is_err());
    }
}
