@echo off
setlocal enabledelayedexpansion
title VrV Desk - Install Virtual Display Driver

echo ========================================================
echo   VrV Desk Virtual Indirect Display Driver Installer
echo ========================================================

:: Check for administrative privileges
net session >nul 2>&1
if %errorLevel% neq 0 (
    echo [!] Administrator privileges required. Relaunching...
    powershell -Command "Start-Process '%~f0' -Verb RunAs"
    exit /b
)

set SCRIPT_DIR=%~dp0
cd /d "%SCRIPT_DIR%"

:: 1. Add driver certificate to TrustedRoot / TrustedPublisher if exists
if exist "%SCRIPT_DIR%IddSampleDriver.cer" (
    echo [*] Installing driver certificate to TrustedPublisher store...
    certutil -addstore "TrustedPublisher" "%SCRIPT_DIR%IddSampleDriver.cer" >nul 2>&1
    certutil -addstore "Root" "%SCRIPT_DIR%IddSampleDriver.cer" >nul 2>&1
)

:: 2. Install driver package using pnputil
echo [*] Installing WDDM IDD Driver package via pnputil...
pnputil /add-driver "%SCRIPT_DIR%IddSampleDriver.inf" /install
if %errorLevel% equ 0 (
    echo [OK] Driver package installed successfully.
) else (
    echo [!] pnputil exited with code %errorLevel%.
)

:: 3. Create virtual device node using devcon or swdevice if present
where devcon.exe >nul 2>&1
if %errorLevel% equ 0 (
    echo [*] Creating root device node Root\VrVDeskIdd...
    devcon.exe install "%SCRIPT_DIR%IddSampleDriver.inf" Root\VrVDeskIdd
)

echo.
echo [*] Virtual Display Driver setup completed.
exit /b 0
