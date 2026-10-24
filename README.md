# webm2gif

以 Rust 和 egui 製作的桌面工具，可批次將 `.webm` 與 `.webp` 轉成 GIF。轉換由 [ImageMagick](https://imagemagick.org/) 執行，介面依系統語系提供繁體中文或英文。

![webm2gif 介面預覽](IMG/view.png)

## 功能

- 遞迴搜尋來源資料夾中的 WebM、WebP 檔案並逐一轉換。
- 可設定輸出高度（100–2000 px），並保留寬高比例。
- 可指定 WebM 影片的開始與結束秒數，只將片段轉換為 GIF。
- 顯示批次進度與轉換結果。
- GIF 儲存在各來源檔案旁的 `gif/` 子資料夾，可從程式開啟輸出位置。
- 支援 Windows、macOS 與 Linux。

## 執行

需要 Rust stable、Cargo、ImageMagick（`magick` 命令）以及桌面圖形環境。
片段轉換功能另需安裝 FFmpeg，並將 `ffmpeg` 加入 PATH。

```bash
git clone https://github.com/cloverdefa/webm2gif.git
cd webm2gif
cargo run --release
```

macOS 可用 `brew install imagemagick` 安裝 ImageMagick；Debian/Ubuntu 可用 `sudo apt install imagemagick`。Windows 請安裝 ImageMagick 並確保 `magick.exe` 位於 PATH。
片段模式支援 WebM，起訖時間以秒輸入；例如開始 `2.5`、結束 `7` 會轉換 2.5 秒至 7 秒的內容。

## 建置

```bash
cargo build --release
```

執行檔位於 `target/release/webm2gif`（Windows 為 `webm2gif.exe`）。GitHub Release 提供 `webm2gif-windows-x64.zip` 可攜式套件，內含應用程式、ImageMagick、FFmpeg 與 FFmpeg 授權檔案；解壓縮後執行 `webm2gif.exe` 即可，無需另外安裝這兩項工具。

## 授權

目前 repository 未包含 LICENSE 檔案；如需使用或散布本專案，請先向作者確認授權。
