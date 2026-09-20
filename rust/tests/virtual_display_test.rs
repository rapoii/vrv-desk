use mirror_core::virtual_display::*;
use std::path::Path;

#[test]
fn test_get_virtual_display_status() {
    let status = get_status();
    println!("Virtual Display Status: {:?}", status);
    assert!(
        !status.modes.is_empty(),
        "Should provide default virtual display modes"
    );
    assert!(
        status
            .modes
            .iter()
            .any(|m| m.width == 1920 && m.height == 1080),
        "Should include 1080p mode"
    );
    assert!(
        status.modes.iter().any(|m| m.refresh_rate == 120),
        "Should include 120Hz mode"
    );
    assert!(!status.driver_name.is_empty());
}

#[test]
fn test_generate_headless_frame_valid_dimensions() {
    let width = 1920;
    let height = 1080;
    let frame = generate_headless_frame(width, height, 42, "Unit Test Headless Frame");

    assert_eq!(
        frame.len(),
        (width * height * 4) as usize,
        "Buffer must match width * height * 4"
    );

    // Ensure buffer has non-zero pixel data (header, border, card background)
    let non_zero_count = frame.iter().take(10000).filter(|&&b| b != 0).count();
    assert!(non_zero_count > 0, "Frame must contain painted graphics");
}

#[test]
fn test_driver_inf_file_validity() {
    let inf_path = Path::new("../driver/virtual_display/IddSampleDriver.inf");
    let alt_inf_path = Path::new("driver/virtual_display/IddSampleDriver.inf");

    let path_to_check = if inf_path.exists() {
        inf_path
    } else {
        alt_inf_path
    };

    assert!(
        path_to_check.exists(),
        "IddSampleDriver.inf must exist in repository"
    );
    let content = std::fs::read_to_string(path_to_check).expect("Failed to read INF file");
    assert!(
        content.contains("[Version]"),
        "INF must have [Version] section"
    );
    assert!(
        content.contains("Class=Display"),
        "INF must specify Class=Display"
    );
    assert!(
        content.contains("Root\\VrVDeskIdd"),
        "INF must define Hardware ID"
    );
}

#[test]
fn test_install_driver_invalid_path_fails_gracefully() {
    let res = install_driver(Some("nonexistent_path_to_driver.inf"));
    assert!(res.is_err(), "Installing nonexistent INF must return Err");
}
