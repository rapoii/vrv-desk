# VrV Desk Virtual Display Driver (Headless PC Support)

## Overview
VrV Desk includes support for Microsoft WDDM 2.5+ Indirect Display Driver (IDD) architecture. This allows remote control of headless Windows PCs (servers, mining rigs, rackmounts, or laptops with lids closed) where no physical monitor is attached.

## Files
- `IddSampleDriver.inf`: INF installation definition file targeting `Root\VrVDeskIdd` and `SWD\VrVDeskIdd`.
- `install.bat`: Automated administrator script to import driver certificates and install driver package via `pnputil`.
- `uninstall.bat`: Clean uninstallation script.

## Resolution Modes Supported
- 1920x1080 @ 60Hz / 120Hz
- 2560x1440 @ 60Hz / 120Hz
- 3840x2160 @ 60Hz

## Headless Fallback Architecture
When running in a headless environment without an installed virtual driver, VrV Desk's `HybridScreenCapturer` engages the **Headless Virtual Canvas Generator**, delivering an active 60 FPS 1080p control stream with telemetry and an interactive prompt to activate hardware virtual displays.
