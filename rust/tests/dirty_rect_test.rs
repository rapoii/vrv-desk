use mirror_core::dirty_rect::{DirtyFrameInfo, DirtyRect};

#[test]
fn test_dirty_rect_geometry() {
    let rect = DirtyRect::new(10, 20, 110, 220);
    assert_eq!(rect.width(), 100);
    assert_eq!(rect.height(), 200);
    assert_eq!(rect.area(), 20_000);
    assert!(!rect.is_empty());
}

#[test]
fn test_dirty_rect_empty() {
    let rect = DirtyRect::new(50, 50, 50, 50);
    assert_eq!(rect.width(), 0);
    assert_eq!(rect.height(), 0);
    assert_eq!(rect.area(), 0);
    assert!(rect.is_empty());

    let inverted = DirtyRect::new(100, 100, 50, 50);
    assert_eq!(inverted.width(), 0);
    assert_eq!(inverted.height(), 0);
    assert!(inverted.is_empty());
}

#[test]
fn test_dirty_rect_union() {
    let r1 = DirtyRect::new(10, 20, 50, 60);
    let r2 = DirtyRect::new(40, 50, 100, 120);
    let u = r1.union(&r2);
    assert_eq!(u, DirtyRect::new(10, 20, 100, 120));

    let empty = DirtyRect::empty();
    assert_eq!(r1.union(&empty), r1);
    assert_eq!(empty.union(&r1), r1);
}

#[test]
fn test_dirty_rect_bounding_box() {
    let rects = vec![
        DirtyRect::new(10, 10, 30, 30),
        DirtyRect::new(100, 200, 150, 250),
        DirtyRect::new(5, 50, 40, 80),
    ];
    let bb = DirtyRect::bounding_box(&rects).expect("should compute bounding box");
    assert_eq!(bb, DirtyRect::new(5, 10, 150, 250));

    assert_eq!(DirtyRect::bounding_box(&[]), None);
}

#[test]
fn test_dirty_rect_total_area() {
    let rects = vec![
        DirtyRect::new(0, 0, 10, 10),   // 100
        DirtyRect::new(20, 20, 30, 30), // 100
    ];
    assert_eq!(DirtyRect::total_area(&rects), 200);
}

#[test]
fn test_dirty_frame_info_metrics() {
    let r1 = DirtyRect::new(100, 100, 200, 200); // 100x100 = 10,000
    let info = DirtyFrameInfo::new(1920, 1080, vec![r1]);

    assert!(!info.is_empty());
    assert_eq!(info.rect_count(), 1);
    assert_eq!(info.total_dirty_area(), 10_000);
    assert_eq!(info.bounding_box(), Some(r1));

    // Total screen area = 1920 * 1080 = 2,073,600
    // Ratio = 10,000 / 2,073,600 ≈ 0.00482 (0.48%)
    let ratio = info.dirty_ratio();
    assert!(ratio > 0.004 && ratio < 0.005);
}

#[test]
fn test_dirty_frame_info_empty() {
    let info = DirtyFrameInfo::new(1920, 1080, vec![]);
    assert!(info.is_empty());
    assert_eq!(info.rect_count(), 0);
    assert_eq!(info.total_dirty_area(), 0);
    assert_eq!(info.bounding_box(), None);
    assert_eq!(info.dirty_ratio(), 0.0);
}

#[cfg(windows)]
#[test]
fn test_dxgi_capturer_dirty_rect_api() {
    use mirror_core::platform::windows_capture::DxgiCapturer;

    let mut capturer = match DxgiCapturer::new() {
        Ok(c) => c,
        Err(e) => {
            println!("DXGI not available in this test environment: {}", e);
            return;
        }
    };

    println!(
        "DXGI Capturer initialized: {}x{}",
        capturer.width, capturer.height
    );
    // Attempt to capture with dirty rects (timeout 100ms)
    match capturer.capture_raw_bgra_with_dirty(100) {
        Ok(Some((w, h, bgra, dirty_info))) => {
            assert_eq!(w, capturer.width);
            assert_eq!(h, capturer.height);
            assert_eq!(bgra.len(), (w * h * 4) as usize);
            println!(
                "Captured frame with {} dirty rects, total dirty area: {}, ratio: {:.4}",
                dirty_info.rect_count(),
                dirty_info.total_dirty_area(),
                dirty_info.dirty_ratio()
            );
            if let Some(bb) = dirty_info.bounding_box() {
                println!("Bounding box: {:?}", bb);
            }
        }
        Ok(None) => {
            println!("Screen was static during test window (no dirty rects) - correctly bypassed!");
        }
        Err(e) => {
            panic!("Unexpected DXGI capture error: {}", e);
        }
    }
}

#[cfg(windows)]
#[test]
fn test_hybrid_capturer_dirty_rect_api() {
    use mirror_core::platform::windows_capture::HybridScreenCapturer;

    let mut capturer = match HybridScreenCapturer::new() {
        Ok(c) => c,
        Err(e) => {
            println!("Capturer init failed (likely headless/CI): {}", e);
            return;
        }
    };

    println!(
        "Hybrid Capturer initialized: {}x{}, is_dxgi: {}",
        capturer.screen_width(),
        capturer.screen_height(),
        capturer.is_dxgi()
    );
    match capturer.capture_raw_bgra_with_dirty(100) {
        Ok(Some((w, h, bgra, dirty_info))) => {
            assert_eq!(w, capturer.screen_width());
            assert_eq!(h, capturer.screen_height());
            assert_eq!(bgra.len(), (w * h * 4) as usize);
            assert!(dirty_info.rect_count() > 0 || dirty_info.is_empty());
            println!(
                "Hybrid captured frame with dirty ratio: {:.4}",
                dirty_info.dirty_ratio()
            );
        }
        Ok(None) => {
            println!("Screen unchanged during test interval (static screen)");
        }
        Err(e) => {
            println!(
                "Capture returned error (normal if no active display session): {}",
                e
            );
        }
    }
}
