#[cfg(windows)]
use mirror_core::{DxgiCapturer, HybridScreenCapturer};

#[test]
#[cfg(windows)]
fn test_dxgi_capturer_initialization_and_capture() {
    let capturer_res = DxgiCapturer::new();
    match capturer_res {
        Ok(mut capturer) => {
            println!(
                "✅ DxgiCapturer initialized successfully: {}x{}",
                capturer.screen_width, capturer.screen_height
            );
            assert!(capturer.screen_width > 0);
            assert!(capturer.screen_height > 0);

            // Acquire next frame with 100ms timeout
            let frame = capturer.capture_jpeg(100, 75, 960);
            match frame {
                Ok(Some(jpeg_bytes)) => {
                    assert!(!jpeg_bytes.is_empty());
                    // Check JPEG SOI marker (0xFF, 0xD8)
                    assert_eq!(jpeg_bytes[0], 0xFF);
                    assert_eq!(jpeg_bytes[1], 0xD8);
                    println!(
                        "✅ Captured DxgiCapturer JPEG frame: {} bytes",
                        jpeg_bytes.len()
                    );
                }
                Ok(None) => {
                    println!("ℹ️ DXGI capture timeout (no screen update during test window), this is normal behavior for AcquireNextFrame");
                }
                Err(e) => {
                    panic!("DxgiCapturer capture_jpeg failed: {:?}", e);
                }
            }
        }
        Err(e) => {
            println!("ℹ️ DxgiCapturer::new() failed on this system (fallback expected): {}", e);
        }
    }
}

#[test]
#[cfg(windows)]
fn test_hybrid_capturer_initialization_and_capture() {
    let mut capturer = HybridScreenCapturer::new().expect("HybridScreenCapturer should initialize");
    println!(
        "HybridScreenCapturer initialized: is_dxgi={}, dimensions={}x{}",
        capturer.is_dxgi(),
        capturer.screen_width(),
        capturer.screen_height()
    );
    assert!(capturer.screen_width() > 0);
    assert!(capturer.screen_height() > 0);

    // Multi-iteration capture loop (5 frames) testing stability & memory
    let mut frames_captured = 0;
    for i in 0..5 {
        // Use timeout_ms = 200
        let frame_res = capturer.capture_jpeg(200, 70, 960);
        match frame_res {
            Ok(Some(jpeg_bytes)) => {
                assert!(!jpeg_bytes.is_empty());
                assert_eq!(jpeg_bytes[0], 0xFF);
                assert_eq!(jpeg_bytes[1], 0xD8);
                println!(
                    "Iteration {}: successfully captured JPEG frame: {} bytes",
                    i + 1,
                    jpeg_bytes.len()
                );
                frames_captured += 1;
            }
            Ok(None) => {
                println!(
                    "Iteration {}: no screen update within timeout (normal for static screen)",
                    i + 1
                );
            }
            Err(e) => {
                panic!("Hybrid capture iteration {} failed: {:?}", i + 1, e);
            }
        }
    }

    println!("Total frames captured: {}/5", frames_captured);
}
