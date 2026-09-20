@echo off
setlocal enabledelayedexpansion
title VrV Desk - Uninstall Virtual Display Driver

echo ========================================================
echo   VrV Desk Virtual Display Driver Uninstaller
echo ========================================================

:: Check for administrative privileges
net session >nul 2>&1
if %errorLevel% neq 0 (
    echo [!] Administrator privileges required. Relaunching...
    powershell -Command "Start-Process '%~f0' -Verb RunAs"
    exit /b
)

echo [*] Searching for installed VrV Desk display driver packages...
for /f "tokens=1,2 delims=:" %%a in ('pnputil /enum-drivers ^| findstr /i "IddSampleDriver"') do (
    echo Found driver: %%b
)

:: Remove device nodes if devcon is available
where devcon.exe >nul 2>&1
if %errorLevel% equ 0 (
    echo [*] Removing device node Root\VrVDeskIdd...
    devcon.exe remove Root\VrVDeskIdd
)

echo [*] Driver uninstall process finished.
exit /b 0
