@echo off
setlocal enabledelayedexpansion
title OpenPet Launcher

echo =======================================================
echo          OpenPet - Desktop Companion Platform         
echo    Acik Kaynak Masaustu Dostu (Windows 11 x64)       
echo =======================================================
echo.

cd /d "%~dp0"

set "TARGET_DIR=%~dp0target\release"
set "HOST_EXE=%TARGET_DIR%\openpet-host.exe"
set "CONTROL_EXE=%TARGET_DIR%\openpet-control.exe"

if not exist "%HOST_EXE%" (
    set "TARGET_DIR=%~dp0target\debug"
    set "HOST_EXE=%TARGET_DIR%\openpet-host.exe"
    set "CONTROL_EXE=%TARGET_DIR%\openpet-control.exe"
)

if not exist "%HOST_EXE%" (
    echo [OpenPet] Binaries not found. Compiling release build...
    echo [OpenPet] Uygulama derleniyor, lutfen bekleyin...
    cargo build --release
    if errorlevel 1 (
        echo [ERROR] Build failed. Please ensure Rust 1.80+ is installed.
        echo [HATA] Derleme basarisiz oldu. Lutfen Rust'in yuklu oldugunu dogrulayin.
        pause
        exit /b 1
    )
    set "TARGET_DIR=%~dp0target\release"
    set "HOST_EXE=%TARGET_DIR%\openpet-host.exe"
    set "CONTROL_EXE=%TARGET_DIR%\openpet-control.exe"
)

echo [OpenPet] Starting background host (desktop pet and system tray)...
echo [OpenPet] Masaustu peti ve sistem tepsisi baslatiliyor...
powershell -NoProfile -WindowStyle Hidden -Command "Start-Process '%HOST_EXE%' -WindowStyle Hidden"

timeout /t 2 /nobreak >nul

echo [OpenPet] Launching Graphical Control Center...
echo [OpenPet] Grafik Kontrol Merkezi aciliyor...
start "" "%CONTROL_EXE%"

echo.
echo [OpenPet] OpenPet is now running!
echo [OpenPet] - Mimi the Cat is floating on your desktop (try dragging or clicking her!)
echo [OpenPet] - System tray icon is active in your taskbar (right-click for menu)
echo [OpenPet] - Control Center window is ready for chat, reminders, and settings
echo.
timeout /t 3 >nul
exit /b 0
