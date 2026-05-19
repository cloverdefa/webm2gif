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
        # 相容不同Python版本
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

lang = "en"

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
        "success": "成功: {}",
        "magick_convert_failed": "ImageMagick 轉換失敗: {}",
        "magick_missing_short": "ImageMagick（magick.exe）缺失，已自動下載和解壓縮。如有問題請手動檢查。",
        "unsupported_type": "不支援的檔案類型: {}",
        "tip": "提示",
        "choose_folder": "請選擇來源資料夾！",
        "not_found_files": "未找到可轉換的檔案！",
        "source_folder": "來源資料夾選擇",
        "choose_folder_btn": "選擇資料夾",
        "param": "參數設定",
        "gif_height": "GIF 高度縮放為",
        "result": "轉換結果",
        "clear": "清空結果",
        "open_gif": "開啟 GIF 資料夾",
        "start": "開始轉換",
        "main_title": "Webm2Gif v1.0 - lilinth/webm2gif"
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
        "magick_missing_msg": "ImageMagick (magick.exe) not found. Download and extract the portable version now? (about 45MB, no installation required after download)",
        "operation_canceled": "Operation canceled",
        "magick_missing_stop": "ImageMagick not found, conversion stopped.",
        "magick_ready": "ImageMagick Ready",
        "magick_ready_info": "ImageMagick has been downloaded and extracted to:\n{}\n\nIf 'magick' command is not found, add this directory to PATH.",
        "magick_not_found": "magick.exe Not Found",
        "magick_not_found_detail": "ImageMagick download/extract error.",
        "error": "Error",
        "not_found_folder": "Folder not found: {}",
        "success": "Success: {}",
        "magick_convert_failed": "ImageMagick Conversion Failed: {}",
        "magick_missing_short": "ImageMagick (magick.exe) missing. Downloaded and extracted automatically. Check manually if issues persist.",
        "unsupported_type": "Unsupported file type: {}",
        "tip": "Tip",
        "choose_folder": "Please select a source folder!",
        "not_found_files": "No convertible files found!",
        "source_folder": "Source Folder",
        "choose_folder_btn": "Choose Folder",
        "param": "Parameters",
        "gif_height": "Resize GIF height to",
        "result": "Results",
        "clear": "Clear",
        "open_gif": "Open GIF Folder",
        "start": "Start Conversion",
        "main_title": "Webm2Gif v1.0 - lilinth/webm2gif"
    }
}
T = L[lang]

# --------- 設定 ----------
VALID_EXTENSIONS = {".webp", ".webm"}
IMAGEMAGICK_ZIP_URL = "https://imagemagick.org/archive/binaries/ImageMagick-7.1.1-47-portable-Q16-x64.zip"
IMAGEMAGICK_ZIP_NAME = "ImageMagick-7.1.1-47-portable-Q16-x64.zip"
IMAGEMAGICK_UNZIP_DIR = "ImageMagick-7.1.1-47-portable-Q16-x64"

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

    if parent is None: parent = tk._default_root
    answer = messagebox.askyesno(
        T["magick_missing_title"], T["magick_missing_msg"], parent=parent
    )
    if not answer:
        messagebox.showinfo(T["operation_canceled"], T["magick_missing_stop"], parent=parent)
        return None
        
    if parent is not None:
        # 假設主視窗有 start_button 全域變數
        try:
            start_button.config(state=tk.NORMAL)
        except Exception:
            pass
            
    def download_with_progress(url, filename):
        win = tk.Toplevel(parent)
        win.title(T["download_title"])
        win.geometry("400x140")
        win.resizable(False, False)
        tk.Label(win, text=T["downloading"]).pack(pady=10)
        bar = ttk.Progressbar(win, length=340, mode="determinate")
        bar.pack(pady=5)
        percent_label = tk.Label(win, text="0%")
        percent_label.pack()
        cancel_flag = {"cancel": False}
        def on_cancel():
            cancel_flag["cancel"] = True
            win.destroy()
        cancel_btn = ttk.Button(win, text=T["cancel_download"], command=on_cancel)
        cancel_btn.pack(pady=15, ipadx=18, ipady=6)
        win.grab_set()
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
                            win.after(0, lambda p=percent: (bar.config(value=p), percent_label.config(text=f"{p}%")))
                result["ok"] = True
                win.after(0, win.destroy)
            except Exception as e:
                win.after(0, lambda: (win.destroy(), messagebox.showerror(T["download_failed"], T["download_failed_detail"].format(e), parent=parent)))
        threading.Thread(target=download_thread, daemon=True).start()
        parent.wait_window(win)
        return result["ok"] and os.path.exists(filename)

    # 下載
    if not os.path.exists(IMAGEMAGICK_ZIP_NAME):
        ok = download_with_progress(IMAGEMAGICK_ZIP_URL, IMAGEMAGICK_ZIP_NAME)
        if not ok:
            return None

    # 解壓縮
    if not os.path.exists(IMAGEMAGICK_UNZIP_DIR):
        try:
            with zipfile.ZipFile(IMAGEMAGICK_ZIP_NAME, 'r') as zip_ref:
                zip_ref.extractall(IMAGEMAGICK_UNZIP_DIR)
        except Exception:
            messagebox.showerror(T["extract_failed"], T["extract_failed_detail"], parent=parent)
            return None

    exe_candidate = find_magick_exe_recursive(IMAGEMAGICK_UNZIP_DIR)
    if exe_candidate:
        messagebox.showinfo(
            T["magick_ready"],
            T["magick_ready_info"].format(exe_candidate),
            parent=parent
        )
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
        return T["unsupported_type"].format(ext)
    input_dir = os.path.dirname(input_filepath)
    gif_dir = os.path.join(input_dir, "gif")
    if not os.path.exists(gif_dir):
        os.makedirs(gif_dir)
    output_filename = os.path.join(gif_dir, os.path.splitext(os.path.basename(input_filepath))[0] + ".gif")

    magick_exe = check_and_download_imagemagick_zip(parent)
    if not magick_exe:
        return T["magick_missing_short"]
    try:
        cmd = [magick_exe, input_filepath]
        if resize_height:
            cmd.extend(["-resize", f"x{resize_height}"])
        cmd.append(output_filename)
        # 關鍵參數：禁止cmd視窗彈出
        creationflags = 0
        if os.name == "nt":
            creationflags = subprocess.CREATE_NO_WINDOW
        subprocess.check_call(cmd, creationflags=creationflags)
        return T["success"].format(os.path.basename(output_filename))
    except Exception as e:
        return T["magick_convert_failed"].format(e)

# --------- 轉換執行緒 ---------
def start_convert_thread(root, label_source_value, text_result, progressbar, open_gif_folder_button, start_button, checkbox_var, spinbox):
    def task():
        start_button.config(state=tk.DISABLED)
        open_gif_folder_button.config(state=tk.DISABLED)
        input_folder = label_source_value.cget("text")
        if not input_folder:
            messagebox.showinfo(T["tip"], T["choose_folder"], parent=root)
            start_button.config(state=tk.NORMAL)
            return

        files_to_convert = []
        for dirpath, _, filenames in os.walk(input_folder):
            for filename in filenames:
                ext = os.path.splitext(filename)[1].lower()
                if ext in VALID_EXTENSIONS:
                    files_to_convert.append(os.path.join(dirpath, filename))

        total_files = len(files_to_convert)
        if total_files == 0:
            messagebox.showinfo(T["tip"], T["not_found_files"], parent=root)
            start_button.config(state=tk.NORMAL)
            return

        progressbar["value"] = 0
        progressbar["maximum"] = total_files

        resize = spinbox.get() if checkbox_var.get() else None

        for idx, input_filepath in enumerate(files_to_convert, 1):
            result = convert_file(input_filepath, resize, parent=root)
            text_result.insert("end", f"{result}\n")
            text_result.see("end")
            progressbar["value"] = idx
            root.update_idletasks()

        open_gif_folder_button.config(state=tk.NORMAL)
        start_button.config(state=tk.NORMAL)

    threading.Thread(target=task, daemon=True).start()

# --------- 清空 ---------
def clean_all(text_result, progressbar, open_gif_folder_button):
    text_result.delete("1.0", "end")
    progressbar["value"] = 0
    open_gif_folder_button.config(state=tk.DISABLED)

# --------- 主介面 ---------
def main():
    root = tk.Tk()
    root.title(T["main_title"])
    root.geometry("560x440")
    root.resizable(False, False)
    root.option_add("*Font", ("微軟正黑體", 11) if lang == "zh" else ("Arial", 11))

    style = ttk.Style()
    style.theme_use('clam')
    style.configure("Accent.TButton", foreground="white", background="#0078d7")

    # 來源資料夾區域
    folder_frame = ttk.LabelFrame(root, text=T["source_folder"])
    folder_frame.grid(row=0, column=0, columnspan=3, padx=15, pady=8, sticky="ew")
    label_source_value = ttk.Label(folder_frame, text="", width=45)
    label_source_value.grid(row=0, column=0, padx=5, pady=5, sticky="w")
    def select_folder():
        folder = filedialog.askdirectory()
        if folder:
            label_source_value.config(text=folder)
    select_file_button_source = ttk.Button(folder_frame, text=T["choose_folder_btn"], command=select_folder)
    select_file_button_source.grid(row=0, column=1, padx=8, pady=5)

    # 參數區域
    param_frame = ttk.LabelFrame(root, text=T["param"])
    param_frame.grid(row=1, column=0, columnspan=3, padx=15, pady=5, sticky="ew")
    checkbox_var = tk.IntVar()
    checkbox_sicle = ttk.Checkbutton(param_frame, variable=checkbox_var, text=T["gif_height"])
    checkbox_sicle.grid(row=0, column=0, padx=5, pady=5)
    spinbox = ttk.Spinbox(param_frame, from_=100, to=1000, width=8)
    spinbox.set(600)
    spinbox.grid(row=0, column=1, padx=5, pady=5)
    ttk.Label(param_frame, text="px").grid(row=0, column=2, padx=5, pady=5)

    # 轉換結果區域
    result_frame = ttk.LabelFrame(root, text=T["result"])
    result_frame.grid(row=2, column=0, columnspan=3, padx=15, pady=5, sticky="nsew")
    text_result = tk.Text(result_frame, width=62, height=10, font=("Consolas", 10))
    text_result.pack(side="left", fill="both", expand=True)
    scrollbar = ttk.Scrollbar(result_frame, command=text_result.yview)
    text_result.config(yscrollcommand=scrollbar.set)
    scrollbar.pack(side="right", fill="y")

    # 進度條
    progressbar = ttk.Progressbar(root, length=500, mode="determinate")
    progressbar.grid(row=3, column=0, columnspan=3, padx=15, pady=12)

    # 按鈕區域
    start_button = ttk.Button(
        root, text=T["start"], style="Accent.TButton",
        command=lambda: start_convert_thread(
            root, label_source_value, text_result, progressbar,
            open_gif_folder_button, start_button, checkbox_var, spinbox
        )
    )
    start_button.grid(row=4, column=0, padx=15, pady=10)

    clean_button = ttk.Button(
        root, text=T["clear"],
        command=lambda: clean_all(text_result, progressbar, open_gif_folder_button)
    )
    clean_button.grid(row=4, column=1, padx=15, pady=10)

    open_gif_folder_button = ttk.Button(
        root, text=T["open_gif"],
        command=lambda: open_gif_folder(label_source_value.cget("text")),
        state=tk.DISABLED
    )
    open_gif_folder_button.grid(row=4, column=2, padx=15, pady=10)

    root.mainloop()

if __name__ == "__main__":
    main()