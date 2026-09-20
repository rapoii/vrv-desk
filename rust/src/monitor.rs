//! Multi-Monitor Enumeration and Management
//! Enumerates active displays via DXGI and provides layout coordinates.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MonitorInfo {
    pub index: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub is_primary: bool,
}

#[cfg(windows)]
pub fn enumerate_monitors() -> Vec<MonitorInfo> {
    use windows::Win32::Graphics::Dxgi::*;

    let mut monitors = Vec::new();
    unsafe {
        if let Ok(factory) = CreateDXGIFactory1::<IDXGIFactory1>() {
            let mut adapter_idx = 0;
            while let Ok(adapter) = factory.EnumAdapters1(adapter_idx) {
                let mut output_idx = 0;
                while let Ok(output) = adapter.EnumOutputs(output_idx) {
                    let mut desc = DXGI_OUTPUT_DESC::default();
                    if output.GetDesc(&mut desc).is_ok() {
                        let w = (desc.DesktopCoordinates.right - desc.DesktopCoordinates.left).abs()
                            as u32;
                        let h = (desc.DesktopCoordinates.bottom - desc.DesktopCoordinates.top).abs()
                            as u32;
                        if desc.AttachedToDesktop.as_bool() && w > 0 && h > 0 {
                            let raw_name = String::from_utf16_lossy(&desc.DeviceName);
                            let clean_name = raw_name.trim_matches('\0').trim().to_string();
                            let is_primary = desc.DesktopCoordinates.left == 0
                                && desc.DesktopCoordinates.top == 0;
                            let name = if clean_name.is_empty() {
                                format!("Display {}", output_idx + 1)
                            } else {
                                format!("Display {} ({})", output_idx + 1, clean_name)
                            };
                            monitors.push(MonitorInfo {
                                index: output_idx,
                                name,
                                width: w,
                                height: h,
                                left: desc.DesktopCoordinates.left,
                                top: desc.DesktopCoordinates.top,
                                right: desc.DesktopCoordinates.right,
                                bottom: desc.DesktopCoordinates.bottom,
                                is_primary,
                            });
                        }
                    }
                    output_idx += 1;
                }
                if !monitors.is_empty() {
                    break;
                }
                adapter_idx += 1;
            }
        }
    }

    if monitors.is_empty() {
        monitors.push(MonitorInfo {
            index: 0,
            name: "Display 1 (Primary)".to_string(),
            width: 1920,
            height: 1080,
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
            is_primary: true,
        });
    }
    monitors
}

#[cfg(not(windows))]
pub fn enumerate_monitors() -> Vec<MonitorInfo> {
    vec![MonitorInfo {
        index: 0,
        name: "Display 1 (Primary)".to_string(),
        width: 1920,
        height: 1080,
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1080,
        is_primary: true,
    }]
}
