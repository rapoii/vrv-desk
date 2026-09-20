# ⚡ VrV Desk

> **Ultra-Lightweight, Low-Latency (<28ms LAN) Bidirectional Screen Mirror & Remote Control for Windows & Android.**  
> Built with **Flutter (Material 3)** and **Rust Core Engine (`mirror_core`)** via `flutter_rust_bridge` v2.

---

## ✨ Features

- **🚀 Ultra-Low Latency & High Frame Rates:** 
  - Sub-28ms end-to-end latency on local Wi-Fi.
  - Target 60 FPS (up to 90/120Hz in Android ADB mode).
  - Zero-copy VRAM capture on Windows using **DXGI Desktop Duplication API**.
- **📱 Dual-Mode Android Control:**
  - **Standalone Mode:** Native `MediaProjection` (hardware H.264/HEVC encoding) + `AccessibilityService` gesture injection (no PC/root/ADB required).
  - **High-Performance ADB Mode:** Direct shell/uinput frame-pipeline for ultra-fast response and high refresh rate displays.
- **🔒 Zero-Cloud & End-to-End Encrypted (E2EE):**
  - No database account, no server tracking, no session recording.
  - Deterministic **6-digit Device ID** derived from Ed25519 identity key (`SHA-256`).
  - Ephemeral **X25519 Diffie-Hellman** key exchange paired with dynamic 6-digit PIN verification.
  - Streaming payload encrypted via **ChaCha20-Poly1305 AEAD** (`VE2E` 28-byte wire framing).
- **🌐 Hybrid Discovery & Connectivity:**
  - **LAN Zero-Config:** UDP Multicast beacon (`239.255.42.99:53210`) automatically detects nearby devices on the same Wi-Fi within milliseconds.
  - **Internet Remote Access:** Stateless Signaling Rendezvous Broker (`vrv_signal`) with RFC 5389 STUN endpoint resolution.
- **🎧 High-Fidelity Internal Audio Streaming:**
  - Internal audio capture on Android 10+ via `AudioPlaybackCaptureConfiguration` (digital capture without room/mic noise).
  - Low-latency WASAPI loopback capture on Windows.
  - Real-time Opus compression (`VAUD` 8-byte binary framing).
- **🎛️ Session Viewer HUD, Controls & Reliability:**
  - Floating Live Diagnostic HUD pill (real-time FPS, Bitrate, Latency, Packet Loss).
  - Dual Input Switcher: Direct Touch vs Trackpad Mode with contextual virtual mouse buttons (`L-Click` / `R-Click`).
  - Streaming Quality Switcher: Eco (720p 30fps), Balanced (1080p 60fps), Ultra (1080p/4K 60fps/120fps).
  - Quick End Session confirmation dialog.
  - Network Watchdog (3.0s buffer timeout with auto-reconnect overlay & exponential backoff).
- **📷 Instant QR Code Pairing:**
  - One-tap pairing via QR camera scanner using `vrvdesk://connect` protocol and structured JSON payloads.
- **💻 Minimal Resource Footprint (Low-End Device Friendly):**
  - CPU usage: `< 6%`
  - RAM footprint: `< 90MB` on Windows
  - GPU hardware-accelerated texture rendering in Flutter without CPU pixel copying.

---

## 🏗️ Architecture Overview

```
┌────────────────────────────────────────────────────────┐
│               VrV Desk Flutter App (UI)                │
│         (Windows Desktop & Android Mobile)             │
└──────────────▲──────────────────────────▲──────────────┘
               │ (Flutter Rust Bridge v2) │ (MethodChannel)
┌──────────────▼──────────────────────────┴──────────────┐
│                    Rust Core Engine                    │
│  ├── Session & State Manager (Device ID, PIN, Pairing) │
│  ├── E2EE Transport (WebSocket + VE2E ChaCha20-Poly1305)│
│  ├── LAN Discovery Engine (UDP Multicast / Broadcast)  │
│  └── E2E Crypto Manager (X25519 + ChaCha20-Poly1305)   │
└──────────────▲──────────────────────────▲──────────────┘
               │                          │
┌──────────────▼──────────┐    ┌──────────▼──────────────┐
│     Windows Native      │    │     Android Native      │
│  ├── DXGI Screen Dupl.  │    │  ├── MediaProjection    │
│  ├── Hardware MFT Enc.  │    │  ├── MediaCodec (H.264) │
│  ├── WASAPI Loopback    │    │  ├── AudioPlaybackCap.  │
│  └── SendInput Driver   │    │  └── Dual Input Engine  │
└─────────────────────────┘    └─────────────────────────┘
```

Detailed architectural specifications and design decisions can be found in [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

---

## 🛠️ Tech Stack

- **UI / Frontend:** Flutter 3.47+ (Dart), Material 3
- **Core Engine:** Rust (Edition 2021)
- **FFI Bridge:** `flutter_rust_bridge` v2
- **Cryptography:** `ed25519-dalek`, `x25519-dalek`, `chacha20poly1305`, `sha2`, `rand`
- **Networking:** `tokio`, `socket2`, `tokio-tungstenite`, UDP Discovery, RFC 5389 STUN
- **Platform Drivers:**
  - Windows: Win32 API (`windows` crate), DXGI Desktop Duplication w/ `GetFrameDirtyRects` idle bypass, Media Foundation MFT Hardware H.264 (NVENC/QSV/AMF), WASAPI Loopback Audio, `SendInput`
  - Android: Kotlin MediaProjection, MediaCodec Hardware H.264, `AudioPlaybackCaptureConfiguration`, InputAccessibilityService, `AdbInputBridge` sub-5ms persistent shell

---

## 🧪 Testing & Verification

The project enforces strict Test-Driven Development (TDD) across all modules (124/124 tests passing):

### Rust Core Unit & Integration Tests (54 tests):
```bash
cd rust
cargo test -- --nocapture
```
**Test suites covered:**
- `identity_test`: Deterministic 6-digit Device ID generation & Ed25519 key verification.
- `pairing_test`: Ephemeral X25519 key exchange & PIN authorization handshake.
- `protocol_test`: Binary wire protocol serialization/deserialization for video (`VH24`), audio (`VAUD`), and input events.
- `discovery_test`: LAN UDP Multicast beacon encoding and peer discovery parsing.
- `platform_test`: Win32 `SendInput` coordinate mapping and DXGI capturer initialization.
- `dirty_rect_test`: Dirty region detection, merging bounding boxes, and idle CPU bypass.
- `mft_h264_test`: Media Foundation hardware encoder pipeline and fallback logic.

### Flutter Widget & Controller Tests (70 tests):
```bash
flutter test
```
**Test suites covered:**
- `DiscoveredDevice` model verification.
- `PinDialog` 6-digit validation, keypad interaction, and submission.
- `HomeView` Device ID card display, scanning state, and connection dispatching.
- `HostModeDialog` audio capability badge, accessibility warning, and ADB mode cards.
- `MirrorView` Live Diagnostic HUD pill, Trackpad virtual mouse buttons, and quality profile switcher.
- `NetworkWatchdog` 3.0s buffer timeout and auto-reconnect overlay triggering.

---

## 🚀 Getting Started

### Prerequisites
- [Flutter SDK](https://flutter.dev) (>= 3.22.0)
- [Rust Toolchain](https://rustup.rs) (`cargo`, `rustc` >= 1.78)
- Android SDK (API 34+) for Android builds
- Visual Studio C++ Build Tools or MinGW-w64 for Windows builds

### Setup
1. Clone the repository:
   ```bash
   git clone https://github.com/rapoii/vrv-desk.git
   cd vrv-desk
   ```
2. Fetch Flutter dependencies:
   ```bash
   flutter pub get
   ```
3. Run test suites:
   ```bash
   flutter test
   cd rust && cargo test
   ```
4. Run on Windows:
   ```bash
   flutter run -d windows
   ```
5. Run on Android:
   ```bash
   flutter run -d <device-id>
   ```

---

## 📄 License
MIT License. Created by Rafi Permana.
