# webm2gif

以 Rust 和 egui 製作的桌面工具，可批次將 `.webm`、`.webp`、`.mp4` 與 `.mov` 轉成 GIF。轉換由 [ImageMagick](https://imagemagick.org/) 和 FFmpeg 執行，介面依系統語系提供繁體中文或英文。

![Webm2Gif 最新介面預覽](assets/demo.gif)

<img src="assets/app-icon.png" width="160" alt="Webm2Gif 動漫點陣 App icon">

## 功能

- 遞迴搜尋來源資料夾中的 WebM、WebP、MP4、MOV 檔案並逐一轉換。
- 可設定輸出高度（100–2000 px），並保留寬高比例。
- 可指定 WebM、MP4 或 MOV 影片的開始與結束秒數，只將片段轉換為 GIF。
- 顯示批次進度與轉換結果。
- GIF 儲存在各來源檔案旁的 `gif/` 子資料夾，可從程式開啟輸出位置。
- 支援 Windows、macOS 與 Linux。

## 執行

需要 Rust stable、Cargo、ImageMagick（`magick` 命令）以及桌面圖形環境。
MP4、MOV 與片段轉換功能另需 FFmpeg；從原始碼執行時請將 `ffmpeg` 加入 PATH。

```bash
git clone https://github.com/cloverdefa/webm2gif.git
cd webm2gif
cargo run --release
```

macOS 可用 `brew install imagemagick` 安裝 ImageMagick；Debian/Ubuntu 可用 `sudo apt install imagemagick`。Windows 請安裝 ImageMagick 並確保 `magick.exe` 位於 PATH。
MP4 與 MOV 由 FFmpeg 轉換，片段模式支援 WebM、MP4 與 MOV，起訖時間以秒輸入；例如開始 `2.5`、結束 `7` 會轉換 2.5 秒至 7 秒的內容。

## 建置

```bash
cargo build --release
```

執行檔位於 `target/release/webm2gif`（Windows 為 `webm2gif.exe`）。GitHub Release 提供的 `webm2gif-windows-x64.zip` 只含 `webm2gif.exe`；ImageMagick 與 FFmpeg 已嵌入執行檔，首次使用時會解壓到系統暫存資料夾，因此無需另外安裝這兩項工具。從原始碼自行建置的版本不會自動附帶這些工具，仍可使用 PATH 中已安裝的版本。

## 授權

目前 repository 未包含 LICENSE 檔案；如需使用或散布本專案，請先向作者確認授權。
