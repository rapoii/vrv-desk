# H.264 Real-Time Video Streaming (Windows Host ➡️ Android MediaCodec) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement low-latency H.264 hardware/accelerated video compression from Windows desktop capture into Android native `MediaCodec` GPU Surface rendering, slashing mirror bandwidth from MB/s to ~150–400 KB/s while maintaining crisp 60 FPS desktop visual fidelity.

**Architecture:** 
- **Rust Host:** Capture DXGI/GDI desktop frames, convert to YUV / Screen Content format, encode using low-latency H.264 encoder (`UsageType::ScreenContentRealTime`, target 2.5–4.0 Mbps CBR/VBR, 60 FPS), and packetize into `VH24` 8-byte headers (`magic: b"VH24"`, `frame_type: u8`, `reserved: u8`, `seq/ts: u16`).
- **Protocol Multiplexing:** WebSocket binary stream multiplexes audio (`VAUD` header), H.264 video (`VH24` header), and legacy JPEG fallback (`0xFF, 0xD8`).
- **Android Decoder & Surface Rendering:** In Kotlin (`H264VideoDecoder.kt` & `MainActivity.kt`), allocate a Flutter `SurfaceTexture` via `flutterEngine.renderer.createSurfaceTexture()`, configure `android.media.MediaCodec` with the Surface, feed incoming NAL units, and render directly to GPU with `releaseOutputBuffer(idx, true)`.
- **Flutter UI Presentation:** Demux `VH24` frames in `MirrorView` / `VideoStreamPlayer`, display via `Texture(textureId: _textureId)` with zero-copy hardware acceleration, and seamless fallback to `Image.memory` if H.264 is unavailable.

**Tech Stack:** Rust (`openh264 = "0.9.8"` ScreenContentRealTime, `tokio`, `image`), Kotlin (`android.media.MediaCodec`, `SurfaceTexture`, `FlutterRenderer`), Flutter (`Texture` widget, MethodChannel).

**Spec:** `docs/ARCHITECTURE.md` (Section 3.1.2 & 3.2.1)

---

## Global Constraints
- Target framerate: 60 FPS with low encoding delay (<15ms).
- Bandwidth target: 150–400 KB/s (1.2–3.2 Mbps) vs uncompressed/JPEG 3–6 MB/s (>85% bandwidth reduction).
- Protocol Header: 8-byte `VH24` packet signature (`b"VH24"`, `frame_type: u8`, `flags: u8`, `timestamp: u16`) followed by Annex B NAL units (`0x00, 0x00, 0x00, 0x01 ...`).
- Fallback resilience: Automatic fallback to JPEG frame stream if H.264 initialization fails.
- Android compatibility: `MediaCodec` ("video/avc") with `SurfaceTexture` zero-copy rendering into Flutter `Texture` widget.

---

### Task 1: Rust H.264 Video Encoder & `VH24` Packetization (Host Engine)

**Files:**
- Create: `rust/src/video.rs`
- Modify: `rust/src/lib.rs`
- Modify: `rust/src/platform/windows_capture.rs`
- Modify: `rust/src/bin/vrv_host.rs`
- Test: `rust/tests/video_h264_test.rs`

**Interfaces:**
- Produces:
  - `pub struct H264VideoEncoder`: initializes OpenH264 ScreenContentRealTime encoder.
  - `pub fn encode_frame(&mut self, width: u32, height: u32, rgb_pixels: &[u8]) -> Result<Option<Vec<u8>>, String>`: outputs Annex-B NALs wrapped in `VH24` 8-byte header.
  - `pub const VIDEO_MAGIC: &[u8; 4] = b"VH24";`
  - `pub const FRAME_TYPE_IDR: u8 = 0x01;`
  - `pub const FRAME_TYPE_DELTA: u8 = 0x02;`
- Consumes:
  - `openh264::encoder::{Encoder, EncoderConfig, BitRate, FrameRate, UsageType, RateControlMode}`
  - Captured frames from `windows_capture.rs` (DXGI/GDI).

- [ ] **Step 1: Write the failing test**
Create `rust/tests/video_h264_test.rs` testing encoder initialization, encoding synthetic 640x360 frames, verifying `VH24` header bytes, verifying NAL start codes `[0, 0, 0, 1]`, and verifying that a delta frame is significantly smaller than the initial keyframe.

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test --test video_h264_test`
Expected: FAIL (module not found).

- [ ] **Step 3: Implement `rust/src/video.rs` and integrate with `windows_capture.rs` & `vrv_host.rs`**
Implement `H264VideoEncoder` with `UsageType::ScreenContentRealTime`, frame buffering, and `VH24` packet wrapper. Add `capture_h264` method in `windows_capture.rs` alongside existing `capture_jpeg`. Update `vrv_host.rs` to stream `VH24` packets with dynamic fallback.

- [ ] **Step 4: Run test to verify it passes**
Run: `cargo test --test video_h264_test` and full `cargo test`
Expected: PASS (all tests pass).

- [ ] **Step 5: Commit**
```bash
git add rust/
git commit -m "feat(rust): implement H.264 video encoder and VH24 packetization"
```

---

### Task 2: Android Native MediaCodec Decoder & Flutter Surface Texture (Mobile Engine)

**Files:**
- Create: `android/app/src/main/kotlin/com/vrvdesk/app/H264VideoDecoder.kt`
- Modify: `android/app/src/main/kotlin/com/vrvdesk/app/MainActivity.kt`
- Create: `lib/src/services/video_stream_player.dart`
- Modify: `lib/src/views/mirror_view.dart`
- Test: `test/video_stream_test.dart`

**Interfaces:**
- Produces:
  - `H264VideoDecoder`: Kotlin class managing `android.media.MediaCodec` ("video/avc") with input queue and Surface output.
  - `MainActivity.kt`: MethodChannel `"com.vrv.desk/video"` supporting `initVideo(width, height) -> textureId`, `feedFrame(bytes)`, and `stopVideo()`.
  - `VideoStreamPlayer`: Dart service handling `VH24` demuxing and forwarding NAL units to native platform channel.
  - `MirrorView`: Renders `Texture(textureId: _textureId)` when active, with fallback to `Image.memory`.

- [ ] **Step 1: Write the failing Dart test**
Create `test/video_stream_test.dart` verifying `VideoStreamPlayer.isVh24Packet()`, parsing header flags, extraction of NAL payload, and channel dispatch.

- [ ] **Step 2: Run Dart test to verify it fails**
Run: `flutter test test/video_stream_test.dart`
Expected: FAIL (class not defined).

- [ ] **Step 3: Implement `H264VideoDecoder.kt`, `MainActivity.kt`, `VideoStreamPlayer`, and `MirrorView`**
Implement hardware video decoding directly to Android `SurfaceTexture`. Integrate `Texture` widget into `MirrorView`.

- [ ] **Step 4: Run tests and verify Android build**
Run: `flutter test` and `flutter build apk --release --split-per-abi`
Expected: 100% tests pass, APK builds cleanly.

- [ ] **Step 5: Commit**
```bash
git add android/ lib/ test/
git commit -m "feat(android/flutter): implement H.264 MediaCodec decoder and SurfaceTexture rendering"
```

---

### Task 3: Live End-to-End Verification on Android Emulator & Release v0.10.0

**Files:**
- Create: `test_e2e_h264_video.py`
- Modify: `docs/ARCHITECTURE.md`

- [ ] **Step 1: Write E2E verification script**
Create `test_e2e_h264_video.py` to connect to `vrv_host.exe`, authenticate via PIN, capture video frames, verify `VH24` header, measure NAL sizes, and calculate real-time compression efficiency.

- [ ] **Step 2: Run live verification against `vrv_host.exe` and Android Emulator**
Install APK on `emulator-5554`, start host with `--pin 482910`, connect emulator, verify live streaming in logcat (`H264VideoDecoder: Decoded NAL frame to Surface`), verify visual quality with screenshot.

- [ ] **Step 3: Update documentation and release v0.10.0**
Compile release binaries, create git tag `v0.10.0`, push to GitHub, and create release.
