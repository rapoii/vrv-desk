# Task 2 Brief: Host Daemon Integration & Smart Delta Frame Loop

## Context & Objectives
You are implementing Task 2 of Phase 8 (DirectX 11 DXGI Desktop Duplication GPU Capture) in `projects/vrv-desk/rust`.
Task 1 implements `HybridScreenCapturer` in `rust/src/platform/windows_capture.rs`. In Task 2, we integrate it into the host streaming daemon in `rust/src/bin/vrv_host.rs` to replace the old CPU-bound GDI BitBlt loop with a high-performance 60-FPS smart delta capture loop.

## Requirements

### 1. Integrate `HybridScreenCapturer` into `rust/src/bin/vrv_host.rs`
- Replace `use mirror_core::gdi_capture::ScreenCapturer;` with `use mirror_core::platform::windows_capture::HybridScreenCapturer;`.
- In `handle_streaming_session`:
  - Initialize capturer: `let mut capturer = HybridScreenCapturer::new().map_err(|e| format!("Capturer init failed: {}", e))?;`.
  - Log active engine:
    ```rust
    if capturer.is_dxgi() {
        println!("🚀 Screen capture engine: DirectX 11 DXGI (Hardware GPU Acceleration)");
    } else {
        println!("⚠️ Screen capture engine: GDI BitBlt (Software CPU Fallback)");
    }
    ```
  - Retrieve dimensions: `screen_w = capturer.screen_width()`, `screen_h = capturer.screen_height()`.
  - In the video streaming task:
    - Target 60 FPS tick (`Duration::from_millis(16)`).
    - Call `capturer.capture_jpeg(10, 60, 1024)`.
    - If `Ok(Some(jpeg_bytes))`: send `Message::Binary(jpeg_bytes.into())` over `ws_sender`.
    - If `Ok(None)`: no new frame presented (static screen), skip sending to conserve CPU and bandwidth.
    - If `Err(e)`: log warning and continue.
  
### 2. Build Release Windows Binary
- Run `cargo build --release --bin vrv_host`.
- Copy binary to `dist/windows/vrv_host.exe`.

### 3. Tests & Verification
- Run `cargo test` across all targets.
- Ensure 0 warnings and clean compilation.

## Deliverables
- Modified `rust/src/bin/vrv_host.rs`
- Compiled `dist/windows/vrv_host.exe`
- Report at `.superpowers/sdd/2026-09-20-dxgi-desktop-duplication-gpu-capture/task-2-report.md`.
