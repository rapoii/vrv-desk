# Implementation Plan: MFT Hardware Video Encoder & E2EE ChaCha20-Poly1305 Transport

**Goal:** Complete the two critical architecture gaps from `docs/ARCHITECTURE.md`:
1. **Milestone 1: Windows Media Foundation (MFT) Hardware Video Encoder**:
   - Hardware-accelerated H.264 video encoding via Windows Media Foundation MFT (`CLSID_CMSH264EncoderMFT` / Intel QuickSync / NVIDIA NVENC / AMD AMF) with D3D11 / NV12 input.
   - Dynamic hardware detection with automatic graceful fallback to OpenH264 ScreenContentRealTime software encoder if hardware MFT is unavailable or fails.
2. **Milestone 2: End-to-End Encryption (E2EE) with ChaCha20-Poly1305**:
   - Derive 256-bit symmetric session key via HKDF-SHA256 from the verified 6-digit PIN handshake token.
   - Implement encrypted packet framing (`VE2E` header: magic, 12-byte nonce, encrypted payload + 16-byte Poly1305 auth tag) in Rust (`rust/src/transport.rs`).
   - Implement matching ChaCha20-Poly1305 AEAD decryptor / encryptor in Flutter (`lib/src/services/e2ee_transport.dart`).
   - Wrap all sensitive streams (`VH24` video, `VAUD` audio, `VINP` keyboard/mouse input) inside E2EE frames.

---

## SDD Task Breakdown

### Task 1: Windows Media Foundation MFT Hardware Encoder Integration
- [ ] 1.1: Design MFT COM wrapper in `rust/src/mft_encoder.rs` using `windows::Win32::Media::MediaFoundation`.
- [ ] 1.2: Implement NV12 conversion and MFT pipeline (MFStartup, CLSID_CMSH264EncoderMFT, ProcessInput, ProcessOutput).
- [ ] 1.3: Expose unified `VideoEncoderEngine` enum in `rust/src/video.rs` (HardwareMFT with OpenH264 fallback).
- [ ] 1.4: Unit test MFT hardware encoder and fallback in `rust/tests/video_h264_test.rs`.
- [ ] 1.5: Verify live E2E streaming with hardware acceleration on Windows host.

### Task 2: E2EE Transport & ChaCha20-Poly1305 Framing (Rust Host)
- [ ] 2.1: Implement HKDF-SHA256 key derivation from PIN authentication token.
- [ ] 2.2: Implement `ChaCha20Poly1305Session` in `rust/src/transport.rs` (`encrypt_packet` & `decrypt_packet` with monotonic 12-byte nonces).
- [ ] 2.3: Unit test round-trip E2EE encryption/decryption in `rust/tests/transport_crypto_test.rs`.
- [ ] 2.4: Integrate E2EE encryption into `rust/src/bin/vrv_host.rs` for video, audio, and input packets.

### Task 3: E2EE Transport & Decryption in Flutter/Android Client
- [ ] 3.1: Add `cryptography` package to `pubspec.yaml`.
- [ ] 3.2: Implement `E2eeTransportService` in `lib/src/services/e2ee_transport.dart`.
- [ ] 3.3: Unit test Dart E2EE decryption in `test/e2ee_transport_test.dart`.
- [ ] 3.4: Integrate E2EE unwrapping into `lib/src/views/mirror_view.dart`.

### Task 4: Full E2E Verification, Benchmark & Release v0.11.0
- [ ] 4.1: Run automated E2E test script validating both MFT hardware encoding and E2EE packet encryption.
- [ ] 4.2: Verify zero regression across entire test suite (Rust + Flutter).
- [ ] 4.3: Build release artifacts (Windows ZIP + Android split APKs), git commit, tag v0.11.0, and publish GitHub Release.
