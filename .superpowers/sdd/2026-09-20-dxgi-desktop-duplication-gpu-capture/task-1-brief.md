# Task 1 Brief: Rust DXGI Desktop Duplication Engine & Hybrid ScreenCapturer

## Context & Objectives
You are implementing Task 1 of Phase 8 (DirectX 11 DXGI Desktop Duplication GPU Capture) in `projects/vrv-desk/rust`.
Currently, `ScreenCapturer` uses GDI `BitBlt` (CPU-bound). We need to implement hardware-accelerated DirectX 11 DXGI Desktop Duplication (`IDXGIOutputDuplication`) in `rust/src/platform/windows_capture.rs` and wrap it in a resilient `HybridScreenCapturer` with automatic GDI fallback.

## Requirements

### 1. `rust/src/platform/windows_capture.rs`
Implement `DxgiCapturer`:
- On Windows:
  - Initializes Direct3D 11 device and immediate context (`D3D11CreateDevice` with `D3D_DRIVER_TYPE_HARDWARE`, `D3D11_CREATE_DEVICE_BGRA_SUPPORT`).
  - Queries `IDXGIDevice`, gets `IDXGIAdapter`, enumerates primary output (`EnumOutputs(0)`), casts to `IDXGIOutput1`, and calls `output1.DuplicateOutput(&device)`.
  - Creates a 2D staging texture (`D3D11_TEXTURE2D_DESC`) with matching screen width and height, `MipLevels: 1`, `ArraySize: 1`, `Format: DXGI_FORMAT_B8G8R8A8_UNORM`, `SampleDesc.Count: 1`, `Usage: D3D11_USAGE_STAGING`, `CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32`.
  - Implements `capture_jpeg(&mut self, timeout_ms: u32, quality: u8, target_width: u32) -> Result<Option<Vec<u8>>, String>`:
    - Calls `self.duplication.AcquireNextFrame(timeout_ms, &mut frame_info, &mut desktop_resource)`.
    - If `DXGI_ERROR_WAIT_TIMEOUT`: returns `Ok(None)` without error.
    - If `DXGI_ERROR_ACCESS_LOST`: attempts to reinitialize duplication. If fails, returns error so caller can fallback.
    - Queries `ID3D11Texture2D` from `desktop_resource`.
    - Copies GPU texture to staging texture: `self.context.CopyResource(&self.staging_texture, &gpu_texture)`.
    - Maps staging texture: `self.context.Map(&self.staging_texture, 0, D3D11_MAP_READ, 0, &mut mapped)`.
    - Reads BGRA pixels from `mapped.pData` considering `mapped.RowPitch`.
    - Converts / compresses to JPEG using `image::codecs::jpeg::JpegEncoder::new_with_quality` (resizing if `target_width < self.screen_width`).
    - Unmaps staging texture: `self.context.Unmap(&self.staging_texture, 0)`.
    - Releases frame: `self.duplication.ReleaseFrame()`.
    - Returns `Ok(Some(jpeg_bytes))`.
- Safe Drop / Cleanup: Ensures acquired frame is released and staging resources are dropped cleanly.
- Non-Windows stub: provides mock structure for other platforms.

Implement `HybridScreenCapturer`:
- Holds `Option<DxgiCapturer>`, `Option<crate::gdi_capture::ScreenCapturer>`, and a boolean `is_dxgi: bool`.
- In `new()`:
  - Attempts `DxgiCapturer::new()`. If successful, prints `🖥️ Initialized DirectX 11 DXGI GPU capture engine` and stores it.
  - If DXGI fails (e.g. running on VM or headless RDP), prints `⚠️ DXGI unavailable, falling back to GDI BitBlt` and creates `ScreenCapturer`.
- Exposes:
  - `pub fn screen_width(&self) -> u32`
  - `pub fn screen_height(&self) -> u32`
  - `pub fn is_dxgi(&self) -> bool`
  - `pub fn capture_jpeg(&mut self, timeout_ms: u32, quality: u8, target_width: u32) -> Result<Option<Vec<u8>>, String>`:
    - If `is_dxgi`: calls `dxgi.capture_jpeg(timeout_ms, quality, target_width)`. On fatal failure, automatically instantiates GDI capturer and switches `is_dxgi = false`.
    - If GDI: calls GDI `capture_jpeg(quality, target_width)` and returns `Ok(Some(bytes))`.

### 2. Export in `rust/src/lib.rs` and `rust/src/platform/mod.rs`
- In `rust/src/platform/mod.rs`: `pub mod windows_capture;`
- In `rust/src/lib.rs`: export `pub mod platform;` and re-export `pub use platform::windows_capture::{DxgiCapturer, HybridScreenCapturer};`.

### 3. Integration Tests in `rust/tests/dxgi_capture_test.rs`
- Write integration tests:
  - Test `HybridScreenCapturer::new()` initializes cleanly and screen width/height are > 0.
  - Test capturing a frame: verify resulting bytes start with JPEG magic bytes `[0xFF, 0xD8]`.
  - Test multiple captures in a loop (5 iterations) verifying stability and no memory leaks.

## Deliverables
- `rust/src/platform/windows_capture.rs`
- `rust/src/platform/mod.rs`
- `rust/src/lib.rs`
- `rust/tests/dxgi_capture_test.rs`
- Report at `.superpowers/sdd/2026-09-20-dxgi-desktop-duplication-gpu-capture/task-1-report.md`.
