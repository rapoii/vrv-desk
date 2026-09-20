# DXGI Dirty Region Detection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement zero-overhead screen change detection via DirectX 11 DXGI `GetFrameDirtyRects` and frame metadata to skip redundant captures/encodes and calculate bounding boxes for partial updates, cutting idle CPU/GPU usage to near zero.

**Architecture:** A pure Rust geometry and metadata module (`dirty_rect.rs`) provides `DirtyRect` structures, union/bounding box calculations, and delta metrics. `DxgiCapturer` queries `DXGI_OUTDUPL_FRAME_INFO.TotalMetadataBufferSize` and calls `GetFrameDirtyRects()` on the output duplication interface. Unchanged frames are discarded before VRAM-to-CPU staging copies, and dirty regions are exposed through `HybridScreenCapturer` and `vrv_host`.

**Tech Stack:** Rust 2021, Windows Direct3D 11 / DXGI Desktop Duplication API (`windows::Win32::Graphics::Dxgi::*`, `windows::Win32::Foundation::RECT`), serde.

**Spec:** `docs/ARCHITECTURE.md` Section 3.1 Item 1: *"Mendeteksi dirty regions (hanya encode area piksel yang berubah) untuk menghemat bandwidth dan CPU di low-end GPU/iGPU (Intel UHD/Iris Xe/AMD Vega)."*

## Global Constraints
- Target platform: Windows 10/11 x86_64, DirectX 11 Feature Level 10.0+.
- Zero breaking changes to existing `HybridScreenCapturer` or `vrv_host` streaming loop.
- Zero CPU copy on static/unchanged frames: `AcquireNextFrame` must release immediately without staging `CopyResource` or `Map` when no pixels have changed (`AccumulatedFrames == 0 && dirty_rects.is_empty()`).
- All tests must pass: 45/45 existing Rust unit/integration tests + new dirty rect test suite.

---

### Task 1: Geometry Data Structures & Bounding Box Logic (`dirty_rect.rs`)

**Files:**
- Create: `rust/src/dirty_rect.rs`
- Modify: `rust/src/lib.rs`
- Test: `rust/tests/dirty_rect_test.rs`

**Interfaces:**
- Consumes: None (pure Rust + serde)
- Produces: `DirtyRect`, `DirtyFrameInfo`, bounding box, area calculation, and subregion helper methods.

- [ ] **Step 1: Write failing unit tests for `DirtyRect` in `rust/tests/dirty_rect_test.rs`**

```rust
use mirror_core::dirty_rect::{DirtyRect, DirtyFrameInfo};

#[test]
fn test_dirty_rect_geometry() {
    let rect = DirtyRect::new(10, 20, 110, 220);
    assert_eq!(rect.width(), 100);
    assert_eq!(rect.height(), 200);
    assert_eq!(rect.area(), 20_000);
    assert!(!rect.is_empty());
}

#[test]
fn test_dirty_rect_bounding_box() {
    let r1 = DirtyRect::new(10, 10, 50, 50);
    let r2 = DirtyRect::new(100, 100, 200, 200);
    let bb = DirtyRect::bounding_box(&[r1, r2]).expect("should have bounding box");
    assert_eq!(bb, DirtyRect::new(10, 10, 200, 200));
}

#[test]
fn test_dirty_frame_info_metrics() {
    let r1 = DirtyRect::new(0, 0, 100, 100); // area 10,000
    let info = DirtyFrameInfo::new(1920, 1080, vec![r1]);
    assert!(!info.is_empty());
    assert!(info.dirty_ratio() < 0.01);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test dirty_rect_test`
Expected: FAIL with "unresolved import `mirror_core::dirty_rect`"

- [ ] **Step 3: Implement `DirtyRect` and `DirtyFrameInfo` in `rust/src/dirty_rect.rs`**

Define `DirtyRect`, `new`, `width`, `height`, `area`, `is_empty`, `union`, `bounding_box`, `total_area`, and `DirtyFrameInfo` with ratio calculations. Export in `rust/src/lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test dirty_rect_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add rust/src/dirty_rect.rs rust/src/lib.rs rust/tests/dirty_rect_test.rs
git commit -m "feat(core): add DirtyRect geometry and DirtyFrameInfo metrics"
```

---

### Task 2: DXGI GetFrameDirtyRects & Unchanged Frame Skip in `windows_capture.rs`

**Files:**
- Modify: `rust/src/platform/windows_capture.rs`
- Modify: `rust/Cargo.toml` (ensure `Win32_Foundation` has `RECT`)

**Interfaces:**
- Consumes: `IDXGIOutputDuplication::GetFrameDirtyRects`, `DXGI_OUTDUPL_FRAME_INFO`
- Produces: `capture_raw_bgra_with_dirty(&mut self, timeout_ms: u32) -> Result<Option<(u32, u32, Vec<u8>, DirtyFrameInfo)>, String>` and dirty-aware skip in `capture_raw_bgra`.

- [ ] **Step 1: Write integration test for dirty rect capture or mock capture**

Add tests in `rust/tests/dirty_rect_test.rs` testing `DxgiCapturer` dirty rect API or mock frame metadata filtering.

- [ ] **Step 2: Run test to verify failure or compile status**

Run: `cargo test --test dirty_rect_test`

- [ ] **Step 3: Implement `GetFrameDirtyRects` in `windows_capture.rs`**

- Read `frame_info.TotalMetadataBufferSize`.
- When `frame_info.AccumulatedFrames == 0 && frame_info.TotalMetadataBufferSize == 0`:
  Release frame immediately and return `Ok(None)`.
- If `TotalMetadataBufferSize > 0`:
  Allocate buffer for `RECT`s, call `self.duplication.GetFrameDirtyRects(...)`, convert to `Vec<DirtyRect>`.
- If dirty rects are empty and `AccumulatedFrames == 0`, skip staging texture copy and return `Ok(None)`.
- Implement `capture_raw_bgra_with_dirty` returning `(width, height, bgra, DirtyFrameInfo)`.

- [ ] **Step 4: Run full test suite to ensure zero regressions**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add rust/src/platform/windows_capture.rs rust/tests/dirty_rect_test.rs
git commit -m "feat(dxgi): implement GetFrameDirtyRects and static frame bypass"
```

---

### Task 3: Expose Dirty Regions in `HybridScreenCapturer` & Live Validation in `vrv_host`

**Files:**
- Modify: `rust/src/platform/windows_capture.rs` (`HybridScreenCapturer`)
- Modify: `rust/src/bin/vrv_host.rs`

**Interfaces:**
- Consumes: `capture_raw_bgra_with_dirty`, `capture_h264_with_dirty`
- Produces: Live logging of dirty area metrics and reduced idle CPU/GPU usage in `vrv_host.exe`.

- [ ] **Step 1: Expose `capture_h264_with_dirty` and `capture_raw_bgra_with_dirty` on `HybridScreenCapturer`**

Add wrapper methods with GDI fallback (GDI returns full screen dirty rect `DirtyRect::new(0, 0, w, h)`).

- [ ] **Step 2: Update `vrv_host.rs` frame streaming task**

Integrate dirty frame info logging on significant updates, confirming dirty rect count and ratio.

- [ ] **Step 3: Test and verify compile**

Run: `cargo test` and `cargo check --bin vrv_host`

- [ ] **Step 4: Commit**

```bash
git add rust/src/platform/windows_capture.rs rust/src/bin/vrv_host.rs
git commit -m "feat(host): integrate dirty region metrics into vrv_host streaming loop"
```
