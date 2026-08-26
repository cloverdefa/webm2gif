# webm2gif

簡單易用的 WebM / WebP 轉 GIF 工具。

`webm2gif` 是一個基於 Python Tkinter 製作的桌面 GUI 工具，可以批次掃描指定資料夾中的 `.webm` 與 `.webp` 檔案，並透過 ImageMagick 將其轉換為 GIF。

## 功能特色

* 支援 `.webm` 與 `.webp` 檔案
* 可批次轉換指定資料夾及其子資料夾中的檔案
* 自動建立 `gif` 輸出資料夾
* 可選擇 GIF 輸出的高度
* 提供轉換進度與轉換紀錄
* 支援中文與英文介面
* 自動依照系統語系切換介面語言
* 本機未安裝 ImageMagick 時，可自動下載可攜版
* ImageMagick 無需額外安裝即可使用
* 提供 Windows 可直接執行的 `.exe` 版本

## 下載

前往 GitHub Releases 下載最新版本：

[下載最新版本](https://github.com/cloverdefa/webm2gif/releases/latest?utm_source=chatgpt.com)

也可以直接下載最新的 Windows 執行檔：

[下載 webm2gif.exe](https://github.com/cloverdefa/webm2gif/releases/latest/download/webm2gif.exe?utm_source=chatgpt.com)

下載後直接執行 `webm2gif.exe` 即可，不需要另外安裝 Python。

## 使用方式

### 1. 啟動程式

直接執行：

```text
webm2gif.exe
```

程式會開啟圖形化操作介面。

### 2. 選擇來源資料夾

點選 **「選擇資料夾」**，選擇包含 WebM 或 WebP 檔案的資料夾。

程式會自動搜尋該資料夾及其子資料夾，並統計找到的可轉換檔案數量。

### 3. 設定輸出高度

預設會保留原始尺寸。

如果需要縮放 GIF，可以啟用 **「縮放高度」**，再設定輸出的高度，例如：

```text
600 px
```

程式會使用 ImageMagick 的 `-resize x600` 方式進行縮放，因此會依照指定高度調整尺寸，同時維持原始寬高比例。

### 4. 開始轉換

點選 **「開始轉換」**。

程式會依序處理找到的 `.webm` 與 `.webp` 檔案，並顯示：

* 目前轉換狀態
* 整體進度
* 每個檔案的轉換結果
* 轉換失敗原因

### 5. 查看輸出結果

轉換完成後，輸出檔案會放在來源檔案所在資料夾的：

```text
gif/
```

例如：

```text
Videos/
├── example.webm
├── animation.webp
└── gif/
    ├── example.gif
    └── animation.gif
```

也可以直接點選 **「開啟輸出資料夾」** 開啟 `gif` 資料夾。

## ImageMagick

`webm2gif` 使用 ImageMagick 執行實際的檔案轉換。

程式啟動轉換時會先尋找系統中的 `magick`：

```text
magick
```

如果找不到，程式會詢問是否自動下載 ImageMagick 可攜版。

目前使用的 ImageMagick 可攜版為：

```text
ImageMagick 7.1.2-29
```

下載的是官方提供的：

```text
ImageMagick-7.1.2-29-portable-Q16-x64.7z
```

程式會將其下載並解壓縮到執行檔所在目錄，不需要修改系統安裝環境。

### 自動下載需求

自動解壓縮 ImageMagick 使用 `py7zr`。

如果執行 Python 原始碼時尚未安裝 `py7zr`，請先執行：

```bash
pip install py7zr
```

Release 提供的 `webm2gif.exe` 已透過 PyInstaller 將相關 Python 相依套件打包，因此一般使用者不需要另外安裝 Python 或 `py7zr`。

## 直接執行 Python 原始碼

如果希望直接執行原始碼，需要 Python 3 環境。

下載 Repository：

```bash
git clone https://github.com/cloverdefa/webm2gif.git
cd webm2gif
```

安裝必要套件：

```bash
pip install py7zr
```

執行：

```bash
python webm2gif.py
```

程式會開啟 GUI。

## 建立 Windows 執行檔

本專案使用 [PyInstaller](https://pyinstaller.org/) 將 Python 程式打包成 Windows `.exe`。

Repository 已提供：

```text
webm2gif.spec
```

可以使用：

```bash
pyinstaller webm2gif.spec
```

完成後，產生的執行檔位於：

```text
dist/webm2gif.exe
```

PyInstaller 設定為 GUI 模式，因此執行時不會另外開啟 Console 視窗。

## 專案結構

```text
webm2gif/
├── .github/
│   └── workflows/
│       ├── auto-merge.yml
│       ├── release.yml
│       └── telegram.yml
├── IMG/
│   └── view.png
├── README.md
├── webm2gif.py
├── webm2gif.spec
└── .gitignore
```

### 主要檔案

| 檔案                                 | 說明                  |
| ---------------------------------- | ------------------- |
| `webm2gif.py`                      | 主要程式與 GUI           |
| `webm2gif.spec`                    | PyInstaller 打包設定    |
| `IMG/view.png`                     | 程式介面預覽圖             |
| `.github/workflows/release.yml`    | Release 自動化流程       |
| `.github/workflows/auto-merge.yml` | Pull Request 自動合併流程 |
| `.github/workflows/telegram.yml`   | GitHub Actions 通知流程 |

Repository 目前包含上述 GitHub Actions 工作流程。

## 支援格式

目前支援：

```text
.webm
.webp
```

程式會遞迴搜尋來源資料夾，因此子資料夾中的檔案也會被納入轉換。

不支援的檔案格式會被略過。

## 轉換流程

整體流程如下：

```text
選擇來源資料夾
        │
        ▼
搜尋 .webm / .webp
        │
        ▼
檢查 ImageMagick
        │
        ├── 已存在
        │
        └── 不存在
              │
              ▼
        詢問是否下載
              │
              ▼
        下載 ImageMagick
              │
              ▼
        解壓縮可攜版
        │
        ▼
逐一轉換檔案
        │
        ▼
輸出至原資料夾 / gif/
```

轉換本身透過 ImageMagick 執行，並可選擇指定 GIF 的輸出高度。

## 多語系

程式會自動偵測系統語系。

目前支援：

* 繁體中文
* English

當系統語系為中文時，使用中文介面；其他語系則使用英文介面。

## 系統需求

### 使用 Release 版本

一般使用者只需要：

* Windows
* `webm2gif.exe`

不需要另外安裝：

* Python
* Tkinter
* PyInstaller
* py7zr

如果系統沒有 ImageMagick，程式可以依照使用者選擇自動下載可攜版。

### 使用 Python 原始碼

需要：

* Python 3
* Tkinter
* `py7zr`

## 注意事項

### ImageMagick 下載

第一次使用且系統沒有 ImageMagick 時，程式可能需要下載 ImageMagick 可攜版。

目前下載的檔案約為數十 MB，實際大小依官方 Release 為準。

下載過程提供進度顯示，也可以取消下載。

### 輸出檔案

GIF 會直接建立在來源檔案所在資料夾的 `gif` 子目錄中。

如果再次執行轉換，相同名稱的 GIF 可能會被重新產生。

### GIF 檔案大小

GIF 相較於 WebM 等現代影片格式通常具有較大的檔案大小。

如果原始影片解析度或幀數較高，轉換後的 GIF 可能會非常大。

建議在需要時啟用 **「縮放高度」**，降低輸出的解析度。

## 技術實作

本專案主要使用：

* Python
* Tkinter
* ImageMagick
* py7zr
* PyInstaller

GUI 使用 Tkinter 建立，轉檔工作會在背景執行緒中進行，以避免轉換過程阻塞使用者介面。

PyInstaller 設定則將 `py7zr` 及相關相依套件納入打包，以支援 Release 版本的獨立執行檔。

## 授權

本 Repository 的授權方式請以 GitHub Repository 中實際提供的 LICENSE 檔案為準。

## 專案連結

[GitHub Repository](https://github.com/cloverdefa/webm2gif)

[GitHub Releases](https://github.com/cloverdefa/webm2gif/releases)

