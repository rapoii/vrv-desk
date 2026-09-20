//! Virtual Display Driver and Headless PC Management
//! Provides support for Indirect Display Drivers (WDDM IDD) and headless canvas fallback.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualDisplayMode {
    pub width: u32,
    pub height: u32,
    pub refresh_rate: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualDisplayStatus {
    pub driver_installed: bool,
    pub driver_name: String,
    pub active: bool,
    pub active_count: u32,
    pub modes: Vec<VirtualDisplayMode>,
    pub is_headless: bool,
    pub physical_monitor_count: u32,
}

pub fn get_supported_modes() -> Vec<VirtualDisplayMode> {
    vec![
        VirtualDisplayMode { width: 1920, height: 1080, refresh_rate: 60 },
        VirtualDisplayMode { width: 1920, height: 1080, refresh_rate: 120 },
        VirtualDisplayMode { width: 2560, height: 1440, refresh_rate: 60 },
        VirtualDisplayMode { width: 3840, height: 2160, refresh_rate: 60 },
    ]
}

/// Find the location of IddSampleDriver.inf on disk
pub fn find_driver_inf() -> Option<PathBuf> {
    let candidate_paths = [
        "driver/virtual_display/IddSampleDriver.inf",
        "../driver/virtual_display/IddSampleDriver.inf",
        "../../driver/virtual_display/IddSampleDriver.inf",
        "IddSampleDriver.inf",
    ];

    for p in &candidate_paths {
        let path = Path::new(p);
        if path.exists() {
            return Some(path.to_path_buf());
        }
    }

    // Try relative to current executable
    if let Ok(mut exe_dir) = std::env::current_exe() {
        exe_dir.pop();
        let p1 = exe_dir.join("driver/virtual_display/IddSampleDriver.inf");
        if p1.exists() {
            return Some(p1);
        }
        let p2 = exe_dir.join("IddSampleDriver.inf");
        if p2.exists() {
            return Some(p2);
        }
    }

    None
}

/// Check if Virtual Display Driver package is installed in Windows driver store
pub fn is_driver_installed() -> bool {
    #[cfg(windows)]
    {
        let output = Command::new("pnputil")
            .args(&["/enum-drivers"])
            .output();

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout).to_lowercase();
            if stdout.contains("iddsampledriver") || stdout.contains("vrvdeskidd") {
                return true;
            }
        }
        false
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Query real-time status of monitors and virtual display driver
pub fn get_status() -> VirtualDisplayStatus {
    let monitors = crate::monitor::enumerate_monitors();
    let total_monitors = monitors.len() as u32;

    let mut virtual_count = 0u32;
    for m in &monitors {
        let name_lower = m.name.to_lowercase();
        if name_lower.contains("virtual") || name_lower.contains("idd") || name_lower.contains("vrv") {
            virtual_count += 1;
        }
    }

    let installed = is_driver_installed();
    let is_headless = total_monitors == 0 || (virtual_count > 0 && virtual_count == total_monitors);

    VirtualDisplayStatus {
        driver_installed: installed,
        driver_name: "VrV Desk Virtual Display (WDDM IDD)".to_string(),
        active: virtual_count > 0,
        active_count: virtual_count,
        modes: get_supported_modes(),
        is_headless,
        physical_monitor_count: total_monitors.saturating_sub(virtual_count),
    }
}

/// Install virtual display driver package via pnputil
pub fn install_driver(inf_path: Option<&str>) -> Result<String, String> {
    #[cfg(windows)]
    {
        let target_inf = if let Some(p) = inf_path {
            PathBuf::from(p)
        } else {
            find_driver_inf().ok_or_else(|| "IddSampleDriver.inf not found in workspace or driver directory".to_string())?
        };

        let inf_str = target_inf.to_str().ok_or_else(|| "Invalid INF path".to_string())?;

        let output = Command::new("pnputil")
            .args(&["/add-driver", inf_str, "/install"])
            .output()
            .map_err(|e| format!("Failed to execute pnputil: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(format!("Driver package installed successfully: {}", stdout.trim()))
        } else {
            Err(format!("pnputil failed (exit {}): {} {}", output.status.code().unwrap_or(-1), stdout, stderr))
        }
    }
    #[cfg(not(windows))]
    {
        Err("Driver installation is only supported on Windows".to_string())
    }
}

/// Uninstall virtual display driver
pub fn uninstall_driver() -> Result<String, String> {
    #[cfg(windows)]
    {
        // First find OEM inf name for IddSampleDriver
        let output = Command::new("pnputil")
            .args(&["/enum-drivers"])
            .output()
            .map_err(|e| format!("Failed to query pnputil: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut target_oem = None;

        let mut current_oem = String::new();
        for line in stdout.lines() {
            let line_trim = line.trim();
            if line_trim.starts_with("Published Name:") || line_trim.starts_with("Nama yang Dipublikasikan:") {
                if let Some(val) = line_trim.split(':').nth(1) {
                    current_oem = val.trim().to_string();
                }
            } else if line_trim.to_lowercase().contains("iddsampledriver") {
                if !current_oem.is_empty() {
                    target_oem = Some(current_oem.clone());
                    break;
                }
            }
        }

        if let Some(oem) = target_oem {
            let del_out = Command::new("pnputil")
                .args(&["/delete-driver", &oem, "/uninstall", "/force"])
                .output()
                .map_err(|e| format!("Failed to delete driver: {}", e))?;

            let del_stdout = String::from_utf8_lossy(&del_out.stdout);
            Ok(format!("Uninstalled driver package {}: {}", oem, del_stdout.trim()))
        } else {
            Ok("No installed VrV Desk virtual display driver found to uninstall".to_string())
        }
    }
    #[cfg(not(windows))]
    {
        Err("Driver uninstallation is only supported on Windows".to_string())
    }
}

/// Generate a high-performance 32-bit BGRA canvas frame for headless PC operation.
/// Used when 0 physical displays are connected and virtual driver is pending.
pub fn generate_headless_frame(width: u32, height: u32, frame_num: u64, _message: &str) -> Vec<u8> {
    let mut buffer = vec![0u8; (width * height * 4) as usize];
    let w = width as usize;
    let h = height as usize;

    // Background color: #0F1217 (B=0x17, G=0x12, R=0x0F, A=0xFF)
    let bg_b = 0x17u8;
    let bg_g = 0x12u8;
    let bg_r = 0x0Fu8;

    for pixel in buffer.chunks_exact_mut(4) {
        pixel[0] = bg_b;
        pixel[1] = bg_g;
        pixel[2] = bg_r;
        pixel[3] = 0xFF;
    }

    // Draw central card
    let card_x1 = w / 6;
    let card_x2 = (w * 5) / 6;
    let card_y1 = h / 4;
    let card_y2 = (h * 3) / 4;

    let card_b = 0x2Bu8;
    let card_g = 0x21u8;
    let card_r = 0x1Au8;

    for y in card_y1..card_y2 {
        for x in card_x1..card_x2 {
            let idx = (y * w + x) * 4;
            // Border check
            if x == card_x1 || x == card_x2 - 1 || y == card_y1 || y == card_y2 - 1 {
                buffer[idx] = 0x50;
                buffer[idx + 1] = 0x40;
                buffer[idx + 2] = 0x30;
                buffer[idx + 3] = 0xFF;
            } else {
                buffer[idx] = card_b;
                buffer[idx + 1] = card_g;
                buffer[idx + 2] = card_r;
                buffer[idx + 3] = 0xFF;
            }
        }
    }

    // Pulsing indicator dot in top-right of card
    let pulse_color = ((frame_num % 60) as f32 / 60.0 * 3.14159).sin();
    let ind_b = (100.0 + pulse_color * 140.0) as u8;
    let ind_g = (150.0 + pulse_color * 90.0) as u8;
    let ind_r = (50.0 + pulse_color * 30.0) as u8;

    let ind_center_x = card_x2 - 40;
    let ind_center_y = card_y1 + 40;
    let radius = 12;

    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy <= radius * radius {
                let px = (ind_center_x as isize + dx) as usize;
                let py = (ind_center_y as isize + dy) as usize;
                if px < w && py < h {
                    let idx = (py * w + px) * 4;
                    buffer[idx] = ind_b;
                    buffer[idx + 1] = ind_g;
                    buffer[idx + 2] = ind_r;
                    buffer[idx + 3] = 0xFF;
                }
            }
        }
    }

    // Draw grid pattern in bottom half of card
    for y in (card_y1 + 100)..(card_y2 - 20) {
        for x in (card_x1 + 30)..(card_x2 - 30) {
            if x % 40 == 0 || y % 40 == 0 {
                let idx = (y * w + x) * 4;
                buffer[idx] = 0x38;
                buffer[idx + 1] = 0x2D;
                buffer[idx + 2] = 0x22;
            }
        }
    }

    buffer
}
