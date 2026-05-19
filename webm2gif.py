import locale
import os
import sys
import subprocess
import threading
import tkinter as tk
from tkinter import ttk, filedialog, messagebox
import shutil
import urllib.request
import zipfile

# ---------- 自動語言偵測 ----------
def detect_lang():
    try:
        lang = None
        try:
            lang = locale.getlocale()[0]
        except Exception:
            pass
        if not lang:
            try:
                lang = locale.setlocale(locale.LC_CTYPE)
            except Exception:
                pass
        lang = str(lang).lower() if lang else ""
        if lang.startswith("zh") or "chinese" in lang:
            return "zh"
    except Exception:
        pass
    return "en"

lang = detect_lang()

L = {
    "zh": {
        "download_title": "正在下載 ImageMagick ...",
        "downloading": "正在下載 ImageMagick ...",
        "cancel_download": "取消下載",
        "download_failed": "下載失敗",
        "download_failed_detail": "自動下載 ImageMagick 失敗。\n{}",
        "extract_failed": "解壓縮異常",
        "extract_failed_detail": "ImageMagick 解壓縮異常。",
        "magick_missing_title": "缺少 ImageMagick",
        "magick_missing_msg": "未偵測到 ImageMagick（magick.exe），是否自動下載並解壓縮便攜版到本機？\n（約45MB，下載後無需安裝）",
        "operation_canceled": "操作已取消",
        "magick_missing_stop": "未偵測到 ImageMagick，轉換已終止。",
        "magick_ready": "ImageMagick 已準備好",
        "magick_ready_info": "已自動下載並解壓縮 ImageMagick 到本機：\n{}\n\n如遇系統找不到 magick 命令，可將該目錄加入 PATH。",
        "magick_not_found": "magick.exe 未找到",
        "magick_not_found_detail": "ImageMagick 下載/解壓縮異常。",
        "error": "錯誤",
        "not_found_folder": "未找到資料夾: {}",
        "success": "✓  {}",
        "magick_convert_failed": "✗  轉換失敗: {}",
        "magick_missing_short": "ImageMagick（magick.exe）缺失，已自動下載和解壓縮。如有問題請手動檢查。",
        "unsupported_type": "不支援的檔案類型: {}",
        "tip": "提示",
        "choose_folder": "請選擇來源資料夾！",
        "not_found_files": "未找到可轉換的檔案！",
        "source_folder": "來源資料夾",
        "choose_folder_btn": "選擇資料夾",
        "param_resize": "縮放高度",
        "result": "轉換紀錄",
        "clear": "清除",
        "open_gif": "開啟輸出資料夾",
        "start": "開始轉換",
        "main_title": "Webm2Gif",
        "subtitle": "影片轉 GIF 工具",
        "no_folder": "尚未選擇資料夾",
        "ready": "就緒",
        "converting": "轉換中…",
        "done": "完成",
        "files_found": "找到 {} 個檔案",
    },
    "en": {
        "download_title": "Downloading ImageMagick ...",
        "downloading": "Downloading ImageMagick ...",
        "cancel_download": "Cancel Download",
        "download_failed": "Download Failed",
        "download_failed_detail": "Failed to download ImageMagick automatically.\n{}",
        "extract_failed": "Extraction Error",
        "extract_failed_detail": "Failed to extract ImageMagick.",
        "magick_missing_title": "ImageMagick Missing",
        "magick_missing_msg": "ImageMagick (magick.exe) not found. Download and extract the portable version now?\n(about 45MB, no installation required)",
        "operation_canceled": "Operation canceled",
        "magick_missing_stop": "ImageMagick not found, conversion stopped.",
        "magick_ready": "ImageMagick Ready",
        "magick_ready_info": "ImageMagick has been downloaded and extracted to:\n{}\n\nIf 'magick' command is not found, add this directory to PATH.",
        "magick_not_found": "magick.exe Not Found",
        "magick_not_found_detail": "ImageMagick download/extract error.",
        "error": "Error",
        "not_found_folder": "Folder not found: {}",
        "success": "✓  {}",
        "magick_convert_failed": "✗  Conversion Failed: {}",
        "magick_missing_short": "ImageMagick (magick.exe) missing. Downloaded and extracted automatically.",
        "unsupported_type": "Unsupported file type: {}",
        "tip": "Tip",
        "choose_folder": "Please select a source folder!",
        "not_found_files": "No convertible files found!",
        "source_folder": "Source Folder",
        "choose_folder_btn": "Browse",
        "param_resize": "Resize height",
        "result": "Conversion Log",
        "clear": "Clear",
        "open_gif": "Open Output Folder",
        "start": "Convert",
        "main_title": "Webm2Gif",
        "subtitle": "Video to GIF Converter",
        "no_folder": "No folder selected",
        "ready": "Ready",
        "converting": "Converting…",
        "done": "Done",
        "files_found": "{} files found",
    }
}
T = L[lang]

# --------- 設定 ----------
VALID_EXTENSIONS = {".webp", ".webm"}
IMAGEMAGICK_ZIP_URL = "https://imagemagick.org/archive/binaries/ImageMagick-7.1.1-47-portable-Q16-x64.zip"
IMAGEMAGICK_ZIP_NAME = "ImageMagick-7.1.1-47-portable-Q16-x64.zip"
IMAGEMAGICK_UNZIP_DIR = "ImageMagick-7.1.1-47-portable-Q16-x64"

# --------- 色票（暖灰 + Teal 強調色）----------
C = {
    "bg":        "#F7F6F3",
    "surface":   "#FFFFFF",
    "border":    "#E2E0D8",
    "border2":   "#C8C6BC",
    "accent":    "#1D9E75",
    "accent_h":  "#0F6E56",
    "accent_bg": "#E1F5EE",
    "text":      "#2C2C2A",
    "text2":     "#5F5E5A",
    "text3":     "#888780",
    "success":   "#3B6D11",
    "success_bg":"#EAF3DE",
    "danger":    "#A32D2D",
    "log_bg":    "#F1EFE8",
    "btn_fg":    "#FFFFFF",
}

# --------- 相容PyInstaller資源路徑 ---------
def resource_path(relative_path):
    if hasattr(sys, '_MEIPASS'):
        return os.path.join(sys._MEIPASS, relative_path)
    return os.path.join(os.path.abspath("."), relative_path)

# --------- ImageMagick 檢查與自動下載 ---------
def check_and_download_imagemagick_zip(parent=None):
    def find_magick_exe_recursive(base_dir):
        for root, dirs, files in os.walk(base_dir):
            if "magick.exe" in files:
                return os.path.join(root, "magick.exe")
        return None

    magick_path = shutil.which("magick")
    if magick_path:
        return magick_path

    exe_candidate = find_magick_exe_recursive(IMAGEMAGICK_UNZIP_DIR)
    if exe_candidate:
        return exe_candidate

    if parent is None:
        parent = tk._default_root
    answer = messagebox.askyesno(
        T["magick_missing_title"], T["magick_missing_msg"], parent=parent
    )
    if not answer:
        messagebox.showinfo(T["operation_canceled"], T["magick_missing_stop"], parent=parent)
        return None

    def download_with_progress(url, filename):
        win = tk.Toplevel(parent)
        win.title(T["download_title"])
        win.geometry("420x160")
        win.resizable(False, False)
        win.configure(bg=C["bg"])
        win.grab_set()

        tk.Label(win, text=T["downloading"],
                 bg=C["bg"], fg=C["text"],
                 font=("Segoe UI", 10)).pack(pady=(20, 8))

        bar = ttk.Progressbar(win, length=360, mode="determinate",
                               style="Accent.Horizontal.TProgressbar")
        bar.pack(pady=4)

        percent_label = tk.Label(win, text="0%",
                                 bg=C["bg"], fg=C["text2"],
                                 font=("Segoe UI", 9))
        percent_label.pack()

        cancel_flag = {"cancel": False}

        def on_cancel():
            cancel_flag["cancel"] = True
            win.destroy()

        tk.Button(win, text=T["cancel_download"],
                  command=on_cancel,
                  bg=C["surface"], fg=C["text"],
                  relief="flat", bd=0,
                  font=("Segoe UI", 9),
                  padx=14, pady=5,
                  cursor="hand2").pack(pady=12)

        result = {"ok": False}

        def download_thread():
            try:
                with urllib.request.urlopen(url) as response, open(filename, 'wb') as out_file:
                    total_length = response.getheader('content-length')
                    if total_length is None:
                        out_file.write(response.read())
                    else:
                        total_length = int(total_length)
                        downloaded = 0
                        block_size = 8192
                        while True:
                            if cancel_flag["cancel"]:
                                try:
                                    out_file.close()
                                    os.remove(filename)
                                except Exception:
                                    pass
                                return
                            buffer = response.read(block_size)
                            if not buffer:
                                break
                            out_file.write(buffer)
                            downloaded += len(buffer)
                            percent = int(downloaded * 100 / total_length)
                            win.after(0, lambda p=percent: (
                                bar.config(value=p),
                                percent_label.config(text=f"{p}%")
                            ))
                result["ok"] = True
                win.after(0, win.destroy)
            except Exception as e:
                win.after(0, lambda: (
                    win.destroy(),
                    messagebox.showerror(T["download_failed"],
                                         T["download_failed_detail"].format(e),
                                         parent=parent)
                ))

        threading.Thread(target=download_thread, daemon=True).start()
        parent.wait_window(win)
        return result["ok"] and os.path.exists(filename)

    if not os.path.exists(IMAGEMAGICK_ZIP_NAME):
        ok = download_with_progress(IMAGEMAGICK_ZIP_URL, IMAGEMAGICK_ZIP_NAME)
        if not ok:
            return None

    if not os.path.exists(IMAGEMAGICK_UNZIP_DIR):
        try:
            with zipfile.ZipFile(IMAGEMAGICK_ZIP_NAME, 'r') as zip_ref:
                zip_ref.extractall(IMAGEMAGICK_UNZIP_DIR)
        except Exception:
            messagebox.showerror(T["extract_failed"], T["extract_failed_detail"], parent=parent)
            return None

    exe_candidate = find_magick_exe_recursive(IMAGEMAGICK_UNZIP_DIR)
    if exe_candidate:
        messagebox.showinfo(T["magick_ready"],
                            T["magick_ready_info"].format(exe_candidate),
                            parent=parent)
        return exe_candidate
    else:
        messagebox.showerror(T["magick_not_found"], T["magick_not_found_detail"], parent=parent)
        return None


# --------- 開啟GIF資料夾 ---------
def open_gif_folder(root_folder):
    folder_path = os.path.join(root_folder, "gif")
    try:
        os.startfile(folder_path)
    except FileNotFoundError:
        messagebox.showerror(T["error"], T["not_found_folder"].format(folder_path))


# --------- 統一用magick轉換 ---------
def convert_file(input_filepath, resize_height, parent=None):
    ext = os.path.splitext(input_filepath)[1].lower()
    if ext not in VALID_EXTENSIONS:
        return T["unsupported_type"].format(ext), "warn"
    input_dir = os.path.dirname(input_filepath)
    gif_dir = os.path.join(input_dir, "gif")
    if not os.path.exists(gif_dir):
        os.makedirs(gif_dir)
    output_filename = os.path.join(
        gif_dir,
        os.path.splitext(os.path.basename(input_filepath))[0] + ".gif"
    )

    magick_exe = check_and_download_imagemagick_zip(parent)
    if not magick_exe:
        return T["magick_missing_short"], "warn"
    try:
        cmd = [magick_exe, input_filepath]
        if resize_height:
            cmd.extend(["-resize", f"x{resize_height}"])
        cmd.append(output_filename)
        creationflags = 0
        if os.name == "nt":
            creationflags = subprocess.CREATE_NO_WINDOW
        subprocess.check_call(cmd, creationflags=creationflags)
        return T["success"].format(os.path.basename(output_filename)), "ok"
    except Exception as e:
        return T["magick_convert_failed"].format(e), "err"


# --------- 轉換執行緒 ---------
def start_convert_thread(app):
    def task():
        app.btn_start.config(state=tk.DISABLED)
        app.btn_open.config(state=tk.DISABLED)
        input_folder = app.folder_var.get()
        if not input_folder or input_folder == T["no_folder"]:
            messagebox.showinfo(T["tip"], T["choose_folder"], parent=app.root)
            app.btn_start.config(state=tk.NORMAL)
            return

        files_to_convert = []
        for dirpath, _, filenames in os.walk(input_folder):
            for filename in filenames:
                ext = os.path.splitext(filename)[1].lower()
                if ext in VALID_EXTENSIONS:
                    files_to_convert.append(os.path.join(dirpath, filename))

        total_files = len(files_to_convert)
        if total_files == 0:
            messagebox.showinfo(T["tip"], T["not_found_files"], parent=app.root)
            app.btn_start.config(state=tk.NORMAL)
            return

        app.progressbar["value"] = 0
        app.progressbar["maximum"] = total_files
        app.status_var.set(T["converting"])

        resize = app.height_var.get() if app.resize_var.get() else None

        for idx, input_filepath in enumerate(files_to_convert, 1):
            msg, kind = convert_file(input_filepath, resize, parent=app.root)
            app.log_insert(msg, kind)
            app.progressbar["value"] = idx
            app.root.update_idletasks()

        app.status_var.set(T["done"])
        app.btn_open.config(state=tk.NORMAL)
        app.btn_start.config(state=tk.NORMAL)

    threading.Thread(target=task, daemon=True).start()


# ======================================================
#  主介面 App 類別
# ======================================================
class App:
    def __init__(self):
        self.root = tk.Tk()
        self.root.title(T["main_title"])
        self.root.geometry("560x590")
        self.root.resizable(False, False)
        self.root.configure(bg=C["bg"])

        self._setup_styles()
        self._build_ui()
        self.root.mainloop()

    # ── TTK 樣式設定 ──────────────────────────────────
    def _setup_styles(self):
        s = ttk.Style()
        s.theme_use("clam")

        s.configure("Accent.Horizontal.TProgressbar",
                     troughcolor=C["border"],
                     background=C["accent"],
                     borderwidth=0,
                     thickness=6)

        s.configure("App.TCheckbutton",
                     background=C["surface"],
                     foreground=C["text"],
                     font=("Segoe UI", 9),
                     focuscolor=C["surface"])
        s.map("App.TCheckbutton",
              background=[("active", C["surface"])],
              foreground=[("active", C["text"])])

        s.configure("App.TSpinbox",
                     fieldbackground=C["surface"],
                     background=C["surface"],
                     foreground=C["text"],
                     bordercolor=C["border2"],
                     lightcolor=C["border"],
                     darkcolor=C["border"],
                     arrowsize=12,
                     font=("Segoe UI", 9))

    # ── Section 標題 ──────────────────────────────────
    def _section_label(self, parent, text):
        tk.Label(parent, text=text.upper() if lang == "en" else text,
                 bg=C["bg"], fg=C["text3"],
                 font=("Segoe UI", 7, "bold"),
                 anchor="w").pack(fill="x", pady=(10, 2))

    # ── 扁平按鈕工廠 ─────────────────────────────────
    def _flat_btn(self, parent, text, command, primary=False):
        bg   = C["accent"]   if primary else C["surface"]
        fg   = C["btn_fg"]   if primary else C["text"]
        h_bg = C["accent_h"] if primary else C["accent_bg"]
        h_fg = C["btn_fg"]   if primary else C["accent"]
        bdr  = C["accent"]   if primary else C["border2"]
        font_w = "bold" if primary else "normal"

        btn = tk.Button(
            parent, text=text, command=command,
            bg=bg, fg=fg,
            relief="flat", bd=0,
            font=("Segoe UI", 9, font_w),
            padx=14, pady=6,
            cursor="hand2",
            highlightbackground=bdr,
            highlightthickness=1,
            activebackground=h_bg,
            activeforeground=h_fg,
        )
        return btn

    # ── 主介面建構 ────────────────────────────────────
    def _build_ui(self):
        root = self.root

        # ━━ 標題列 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        header = tk.Frame(root, bg=C["surface"],
                          highlightbackground=C["border"],
                          highlightthickness=1)
        header.pack(fill="x")

        lhs = tk.Frame(header, bg=C["surface"])
        lhs.pack(side="left", padx=20, pady=14)

        tk.Label(lhs, text=T["main_title"],
                 bg=C["surface"], fg=C["text"],
                 font=("Segoe UI", 15, "bold")).pack(anchor="w")
        tk.Label(lhs, text=T["subtitle"],
                 bg=C["surface"], fg=C["text3"],
                 font=("Segoe UI", 9)).pack(anchor="w")

        badge_frame = tk.Frame(header, bg=C["accent_bg"],
                               highlightbackground=C["accent"],
                               highlightthickness=1)
        badge_frame.pack(side="right", padx=20)
        tk.Label(badge_frame, text="v 1.0",
                 bg=C["accent_bg"], fg=C["accent"],
                 font=("Segoe UI", 8, "bold"),
                 padx=8, pady=3).pack()

        # ━━ 捲動主體 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        body = tk.Frame(root, bg=C["bg"])
        body.pack(fill="both", expand=True, padx=20, pady=(8, 0))

        # ── 來源資料夾卡 ─────────────────────────────
        self._section_label(body, T["source_folder"])

        folder_card = tk.Frame(body, bg=C["surface"],
                               highlightbackground=C["border"],
                               highlightthickness=1)
        folder_card.pack(fill="x")

        fc_inner = tk.Frame(folder_card, bg=C["surface"])
        fc_inner.pack(fill="x", padx=12, pady=10)

        self.folder_var = tk.StringVar(value=T["no_folder"])
        self.lbl_folder = tk.Label(
            fc_inner, textvariable=self.folder_var,
            bg=C["surface"], fg=C["text3"],
            font=("Segoe UI", 9),
            anchor="w", width=44
        )
        self.lbl_folder.pack(side="left", fill="x", expand=True)

        def _on_folder_change(*_):
            v = self.folder_var.get()
            self.lbl_folder.config(
                fg=C["text"] if v != T["no_folder"] else C["text3"]
            )
        self.folder_var.trace_add("write", _on_folder_change)

        self._flat_btn(fc_inner, T["choose_folder_btn"],
                       self._select_folder).pack(side="right")

        # ── 參數卡 ───────────────────────────────────
        self._section_label(body, T["param_resize"])

        param_card = tk.Frame(body, bg=C["surface"],
                              highlightbackground=C["border"],
                              highlightthickness=1)
        param_card.pack(fill="x")

        pc_inner = tk.Frame(param_card, bg=C["surface"])
        pc_inner.pack(fill="x", padx=12, pady=10)

        self.resize_var = tk.BooleanVar(value=False)
        ttk.Checkbutton(pc_inner,
                        variable=self.resize_var,
                        text=T["param_resize"],
                        style="App.TCheckbutton",
                        command=self._toggle_spinbox).pack(side="left")

        self.height_var = tk.StringVar(value="600")
        self.spinbox = ttk.Spinbox(
            pc_inner, from_=100, to=2000,
            width=7, textvariable=self.height_var,
            style="App.TSpinbox", state="disabled"
        )
        self.spinbox.pack(side="left", padx=(10, 4))
        tk.Label(pc_inner, text="px",
                 bg=C["surface"], fg=C["text3"],
                 font=("Segoe UI", 9)).pack(side="left")

        # ── 紀錄區 ───────────────────────────────────
        self._section_label(body, T["result"])

        log_outer = tk.Frame(body, bg=C["log_bg"],
                             highlightbackground=C["border"],
                             highlightthickness=1)
        log_outer.pack(fill="both", expand=True)

        self.log_text = tk.Text(
            log_outer,
            bg=C["log_bg"], fg=C["text"],
            font=("Consolas", 9),
            relief="flat", bd=0,
            wrap="word",
            padx=10, pady=8,
            height=9,
            cursor="arrow",
            state="disabled",
            selectbackground=C["accent_bg"],
        )
        self.log_text.pack(side="left", fill="both", expand=True)
        self.log_text.tag_configure("ok",   foreground=C["success"])
        self.log_text.tag_configure("err",  foreground=C["danger"])
        self.log_text.tag_configure("warn", foreground=C["text2"])

        sb = tk.Scrollbar(log_outer, command=self.log_text.yview,
                          bg=C["log_bg"], troughcolor=C["log_bg"],
                          relief="flat", bd=0, width=10)
        self.log_text.config(yscrollcommand=sb.set)
        sb.pack(side="right", fill="y")

        # ━━ 底部工具列 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        footer = tk.Frame(root, bg=C["surface"],
                          highlightbackground=C["border"],
                          highlightthickness=1)
        footer.pack(fill="x", side="bottom")

        # 左：狀態列 + 進度條
        left_bar = tk.Frame(footer, bg=C["surface"])
        left_bar.pack(side="left", padx=16, pady=12, fill="x", expand=True)

        self.status_var = tk.StringVar(value=T["ready"])
        tk.Label(left_bar, textvariable=self.status_var,
                 bg=C["surface"], fg=C["text3"],
                 font=("Segoe UI", 8)).pack(anchor="w")

        self.progressbar = ttk.Progressbar(
            left_bar, length=220, mode="determinate",
            style="Accent.Horizontal.TProgressbar"
        )
        self.progressbar.pack(anchor="w", pady=(4, 0))

        # 右：按鈕群
        btn_bar = tk.Frame(footer, bg=C["surface"])
        btn_bar.pack(side="right", padx=16, pady=10)

        self.btn_open = self._flat_btn(
            btn_bar, T["open_gif"],
            lambda: open_gif_folder(self.folder_var.get())
        )
        self.btn_open.config(state=tk.DISABLED)
        self.btn_open.pack(side="left", padx=(0, 6))

        self._flat_btn(btn_bar, T["clear"],
                       self._clear_log).pack(side="left", padx=(0, 6))

        self.btn_start = self._flat_btn(
            btn_bar, T["start"],
            lambda: start_convert_thread(self),
            primary=True
        )
        self.btn_start.pack(side="left")

    # ── 選擇資料夾 ────────────────────────────────────
    def _select_folder(self):
        folder = filedialog.askdirectory()
        if folder:
            self.folder_var.set(folder)
            count = sum(
                1 for _, _, fs in os.walk(folder)
                for f in fs
                if os.path.splitext(f)[1].lower() in VALID_EXTENSIONS
            )
            self.status_var.set(T["files_found"].format(count))

    # ── 啟用/停用 Spinbox ────────────────────────────
    def _toggle_spinbox(self):
        self.spinbox.config(
            state="normal" if self.resize_var.get() else "disabled"
        )

    # ── 寫入紀錄 ─────────────────────────────────────
    def log_insert(self, msg, kind="ok"):
        self.log_text.config(state="normal")
        self.log_text.insert("end", msg + "\n", kind)
        self.log_text.see("end")
        self.log_text.config(state="disabled")

    # ── 清空 ─────────────────────────────────────────
    def _clear_log(self):
        self.log_text.config(state="normal")
        self.log_text.delete("1.0", "end")
        self.log_text.config(state="disabled")
        self.progressbar["value"] = 0
        self.status_var.set(T["ready"])
        self.btn_open.config(state=tk.DISABLED)


# ─── 入口 ────────────────────────────────────────────
if __name__ == "__main__":
    App()