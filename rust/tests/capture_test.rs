#[cfg(windows)]
use mirror_core::gdi_capture::ScreenCapturer;

#[test]
#[cfg(windows)]
fn test_screen_capture_jpeg() {
    let capturer = ScreenCapturer::new().expect("Capturer should initialize");
    assert!(capturer.screen_width > 0);
    assert!(capturer.screen_height > 0);

    let jpeg_bytes = capturer
        .capture_jpeg(60, 960)
        .expect("Capture should succeed");
    assert!(!jpeg_bytes.is_empty());
    // Check JPEG SOI marker (0xFF, 0xD8)
    assert_eq!(jpeg_bytes[0], 0xFF);
    assert_eq!(jpeg_bytes[1], 0xD8);
    println!("Captured JPEG frame: {} bytes", jpeg_bytes.len());
}
