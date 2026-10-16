# webm2gif

以 Python 和 Tkinter 製作的桌面工具，可批次將 `.webm` 與 `.webp` 檔案轉成 GIF。程式使用 [ImageMagick](https://imagemagick.org/) 執行轉換，提供繁體中文與英文介面。

![webm2gif 介面預覽](IMG/view.png)

## 功能

- 遞迴搜尋選定資料夾中的 `.webm`、`.webp` 檔案並逐一轉換。
- 可選擇輸出高度，並維持原始寬高比例；預設保留原尺寸。
- 顯示批次進度、轉換結果與錯誤紀錄。
- 成功輸出後可從程式開啟輸出資料夾。
- 依系統語系使用繁體中文或英文介面。
- Windows 上找不到 ImageMagick 時，可選擇下載並解壓縮 Windows x64 可攜版。

## 下載 Windows 版本

到 [GitHub Releases](https://github.com/cloverdefa/webm2gif/releases/latest) 下載 `webm2gif.exe`，下載後直接執行即可。此版本不需另外安裝 Python；若找不到 ImageMagick，程式會詢問是否下載可攜版。

## 使用方式

1. 執行程式，按 **選擇資料夾** 選取來源資料夾。
2. 如需縮放，勾選 **縮放高度** 並設定高度（介面範圍為 100–2000 px）。未勾選時保留原尺寸。
3. 按 **開始轉換**，在紀錄區查看進度與結果。
4. 按 **開啟輸出資料夾** 查看成功轉換的檔案。

每個 GIF 會存放在對應來源檔案所在目錄的 `gif/` 子資料夾。例如來源檔位於不同子資料夾時，各子資料夾會各自建立 `gif/`：

```text
Videos/
├── clip.webm
├── gif/
│   └── clip.gif
└── subfolder/
    ├── animation.webp
    └── gif/
        └── animation.gif
```

同名 GIF 若已存在，轉換時會由 ImageMagick 覆寫。

## 從原始碼執行

需要 Python 3、Tkinter，以及已安裝的 ImageMagick（`magick` 命令）。若在 Windows 上要使用程式內的可攜版下載與解壓縮功能，另需 `py7zr`。

```bash
git clone https://github.com/cloverdefa/webm2gif.git
cd webm2gif
python -m pip install py7zr
python webm2gif.py
```

如果不需要 Windows 可攜版自動下載，可略過 `py7zr`。macOS 或 Linux 找不到 ImageMagick 時，請透過該平台的套件管理器安裝後重新啟動程式；程式不會在這些平台下載 Windows 版本。

## 建立 Windows 執行檔

安裝 PyInstaller 與 `py7zr`，再從 Windows 環境執行：

```bash
python -m pip install pyinstaller py7zr
pyinstaller webm2gif.spec
```

執行檔會產生於 `dist/webm2gif.exe`。`webm2gif.spec` 會收集 `py7zr` 及其可用相依套件，並以無主控台視窗模式打包。

## ImageMagick 與轉換方式

程式會先查找系統中的 `magick` 命令；在 Windows 也會查找先前下載到程式目錄的 ImageMagick 可攜版。若 Windows 找不到 ImageMagick，程式可下載官方 ImageMagick 7.1.2-29 的 `portable-Q16-x64.7z` 套件，並解壓縮到程式目錄。使用 Python 原始碼且需要此功能時，必須安裝 `py7zr`。

轉換由 ImageMagick 執行。指定高度時，程式使用 `-resize x高度`，維持寬高比例；GIF 輸出在來源檔案旁的 `gif/` 目錄。

## 專案檔案

| 檔案 | 用途 |
| --- | --- |
| `webm2gif.py` | Tkinter 介面、檔案搜尋、ImageMagick 準備與轉換流程 |
| `webm2gif.spec` | PyInstaller 打包設定 |
| `IMG/view.png` | 介面預覽圖 |
| `.github/workflows/` | Release、PR 自動合併與 Telegram 通知工作流程 |

## 授權

目前 repository 未包含 LICENSE 檔案；如需使用或散布本專案，請先向作者確認授權。
