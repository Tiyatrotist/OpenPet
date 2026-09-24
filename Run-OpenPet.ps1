# OpenPet Windows 11 Desktop Companion Launcher
# Maintained by Tiyatrotist - AGPL-3.0

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
Set-Location $ScriptDir

Write-Host "=======================================================" -ForegroundColor Cyan
Write-Host "         OpenPet - Desktop Companion Platform          " -ForegroundColor Yellow
Write-Host "   Acik Kaynak Masaustu Dostu (Windows 11 x64)        " -ForegroundColor Cyan
Write-Host "=======================================================" -ForegroundColor Cyan
Write-Host ""

$HostExe = Join-Path $ScriptDir "target\release\openpet-host.exe"
$ControlExe = Join-Path $ScriptDir "target\release\openpet-control.exe"

if (-not (Test-Path $HostExe)) {
    $HostExe = Join-Path $ScriptDir "target\debug\openpet-host.exe"
    $ControlExe = Join-Path $ScriptDir "target\debug\openpet-control.exe"
}

if (-not (Test-Path $HostExe)) {
    Write-Host "[OpenPet] Binaries not found. Building release version..." -ForegroundColor Yellow
    cargo build --release
    $HostExe = Join-Path $ScriptDir "target\release\openpet-host.exe"
    $ControlExe = Join-Path $ScriptDir "target\release\openpet-control.exe"
}

Write-Host "[OpenPet] Launching OpenPet Host (Pet Window & Taskbar Tray)..." -ForegroundColor Green
Start-Process -FilePath $HostExe -WindowStyle Hidden

Start-Sleep -Seconds 2

Write-Host "[OpenPet] Launching OpenPet Control Center..." -ForegroundColor Green
Start-Process -FilePath $ControlExe

Write-Host ""
Write-Host "OpenPet is running successfully!" -ForegroundColor Green
Write-Host "- Mimi is floating on your desktop (try dragging or clicking her!)" -ForegroundColor Cyan
Write-Host "- System tray icon is active in taskbar (right-click for menu)" -ForegroundColor Cyan
Write-Host "- Control Center window is ready" -ForegroundColor Cyan
