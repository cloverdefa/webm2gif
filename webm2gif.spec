# -*- mode: python ; coding: utf-8 -*-
from PyInstaller.utils.hooks import collect_all, copy_metadata

datas = []
binaries = []
hiddenimports = []

# py7zr 及其「可能」用到的相依套件；用 try/except 容錯，
# 因為不同 py7zr 版本 / python 版本實際安裝的子依賴會不同
_candidates = [
    'py7zr', 'pyppmd', 'pybcj', 'pyzstd', 'brotli', 'brotlicffi',
    'inflate64', 'multivolumefile', 'texttable', 'psutil',
    'pycryptodomex', 'backports.zstd',
]

for pkg in _candidates:
    try:
        d, b, h = collect_all(pkg)
        datas += d
        binaries += b
        hiddenimports += h
    except Exception:
        pass  # 該套件未安裝或非套件形式，略過

for pkg in _candidates:
    try:
        datas += copy_metadata(pkg)
    except Exception:
        pass  # 找不到 metadata 就略過

a = Analysis(
    ['webm2gif.py'],
    pathex=[],
    binaries=binaries,
    datas=datas,
    hiddenimports=hiddenimports,
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[],
    noarchive=False,
    optimize=0,
)
pyz = PYZ(a.pure)
exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.datas,
    name='webm2gif',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    console=False,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
)
