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
  - Streaming payload encrypted via **ChaCha20-Poly1305** and DTLS-SRTP.
- **🌐 Hybrid Discovery & Connectivity:**
  - **LAN Zero-Config:** UDP Multicast beacon (`239.255.42.99:53210`) automatically detects nearby devices on the same Wi-Fi within milliseconds.
  - **Internet Remote Access:** Direct WebRTC P2P (ICE / STUN) with transparent NAT hole punching.
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
│  ├── WebRTC / P2P Transport (DataChannel & MediaTrack) │
│  ├── LAN Discovery Engine (UDP Multicast / mDNS)       │
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
- **Networking:** `tokio`, `socket2`, UDP Multicast, WebRTC
- **Platform Drivers:**
  - Windows: Win32 API (`windows` crate), DXGI Desktop Duplication, `SendInput`
  - Android: Kotlin MediaProjection, MediaCodec, AccessibilityService

---

## 🧪 Testing & Verification

The project enforces strict Test-Driven Development (TDD) across all modules:

### Rust Core Unit & Integration Tests:
```bash
cd rust
cargo test -- --nocapture
```
**Test suites covered:**
- `identity_test`: Deterministic 6-digit Device ID generation & Ed25519 key verification.
- `pairing_test`: Ephemeral X25519 key exchange & PIN authorization handshake.
- `protocol_test`: Binary wire protocol serialization/deserialization for video, audio, and input events.
- `discovery_test`: LAN UDP Multicast beacon encoding and peer discovery parsing.
- `platform_test`: Win32 `SendInput` coordinate mapping and DXGI capturer initialization.

### Flutter Widget & Controller Tests:
```bash
flutter test
```
**Test suites covered:**
- `DiscoveredDevice` model verification.
- `PinDialog` 6-digit validation and submission.
- `HomeView` Device ID card display, scanning state, and connection dispatching.

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
