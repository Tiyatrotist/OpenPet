@echo off
setlocal enabledelayedexpansion
title OpenPet Launcher

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
    echo =======================================================
    echo          OpenPet - Desktop Companion Platform         
    echo    Acik Kaynak Masaustu Dostu (Windows 11 x64)       
    echo =======================================================
    echo.
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

rem Launch host silently in the background via VBScript
if exist "%~dp0OpenPet.vbs" (
    wscript.exe "%~dp0OpenPet.vbs"
) else (
    start "" /b "%HOST_EXE%"
)

rem If user specifically requested control center via command line flag
if "%~1"=="--control" (
    start "" "%CONTROL_EXE%"
) else if "%~1"=="-c" (
    start "" "%CONTROL_EXE%"
)

exit /b 0
