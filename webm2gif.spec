# -*- mode: python ; coding: utf-8 -*-
from PyInstaller.utils.hooks import collect_all, copy_metadata

datas = []
binaries = []
hiddenimports = []

# py7zr 及其相依套件：一次收集模組、二進位檔、資料檔
for pkg in ['py7zr', 'pyppmd', 'pybcj', 'pyzstd', 'brotli',
            'inflate64', 'multivolumefile', 'texttable', 'psutil']:
    d, b, h = collect_all(pkg)
    datas += d
    binaries += b
    hiddenimports += h

# py7zr 用 importlib.metadata 讀套件版本，需要把 dist-info 一併帶入
for pkg in ['py7zr', 'pyppmd', 'pybcj', 'pyzstd', 'brotli',
            'inflate64', 'multivolumefile', 'texttable', 'psutil',
            'pycryptodomex']:
    datas += copy_metadata(pkg)

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
