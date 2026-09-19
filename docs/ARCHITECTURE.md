# Specification: Ultra-Lightweight Bidirectional Screen Mirror & Remote Control (Windows & Android)

- **Date:** 2026-09-19
- **Status:** Approved / In Review
- **Target Platforms:** Windows 10/11 (Native Desktop) & Android 8.0+ (Mobile)
- **Primary Tech Stack:** Rust (Core Engine) + Flutter (UI Layer via `flutter_rust_bridge` v2)

---

## 1. Executive Summary & Goals

Aplikasi screen mirror dan remote control dua arah (PC mengontrol Android dan Android mengontrol PC) yang dirancang khusus untuk **perangkat low-end**:
- **Ultra-rendah latensi (<30ms pada jaringan LAN)**.
- **Konsumsi resource minimal** (CPU utilization < 10%, RAM < 120MB, zero-copy VRAM to encoder pipeline).
- **Modern & Bebas Hambatan:** Tanpa iklan, tanpa batasan durasi waktu sesi, tanpa keharusan membuat akun cloud terpusat.
- **Hybrid Networking:** Zero-config mDNS/UDP auto-discovery di jaringan Wi-Fi lokal, serta P2P WebRTC NAT Traversal (STUN) saat terhubung lewat Internet publik.

---

## 2. High-Level System Architecture

Sistem menggunakan pola arsitektur **Shared Native Core** dengan **Flutter Presentation**:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        Flutter UI Presentation                         │
│             (Windows Desktop App & Android APK / Mobile App)           │
└───────────────────▲────────────────────────────────▲───────────────────┘
                    │ flutter_rust_bridge v2         │ Platform Channels
┌───────────────────▼────────────────────────────────┴───────────────────┐
│                           Rust Core Engine                             │
│  ├── Session & State Manager (Device ID, One-Time PIN, QR Token)       │
│  ├── WebRTC / P2P Transport (DataChannel for input, MediaStream tracks)│
│  ├── LAN Discovery Service (UDP Broadcast / Multicast on port 53210)   │
│  └── E2EE Cryptography Engine (Ed25519, X25519, ChaCha20-Poly1305)     │
└───────────────────▲────────────────────────────────▲───────────────────┘
                    │                                │
┌───────────────────▼───────────┐        ┌───────────▼──────────────────┐
│      Windows Driver Stack     │        │     Android Native Stack     │
│  ├── DXGI Desktop Duplication │        │  ├── MediaProjection Service │
│  ├── Media Foundation Enc MFT │        │  ├── MediaCodec (H.264 NAL)  │
│  ├── WASAPI Audio Loopback    │        │  ├── AudioPlaybackCapture    │
│  └── Win32 SendInput Pipeline │        │  ├── Accessibility Service   │
│                               │        │  └── ADB Shell Engine        │
└───────────────────────────────┘        └──────────────────────────────┘
```

---

## 3. Platform Subsystems

### 3.1. Windows Subsystem (Desktop Host & Client)
1. **Screen Capture Engine (DXGI Desktop Duplication):**
   - Mengambil frame desktop langsung dari VRAM GPU via DirectX 11 / DXGI Desktop Duplication API (`IDXGIOutputDuplication`).
   - Mendeteksi dirty regions (hanya encode area piksel yang berubah) untuk menghemat bandwidth dan CPU di low-end GPU/iGPU (Intel UHD/Iris Xe/AMD Vega).
2. **Hardware Video Encoder:**
   - Pipeline Windows Media Foundation (MFT) dengan pemilihan encoder otomatis berbasis prioritas:
     1. Intel QuickSync Video (`MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS`)
     2. NVIDIA NVENC
     3. AMD AMF
     4. Fallback: Software H.264 (`openh264` multithreaded dengan preset ultrafast / low-delay).
   - Format: H.264 Baseline/Main Profile, NAL format Annex B, CBR/VBR adaptif dengan target latency sub-15ms encoding.
3. **Audio Capture:**
   - Windows WASAPI Loopback Capture (`IAudioClient` diaktifkan dengan flag `AUDCLNT_STREAMFLAGS_LOOPBACK`).
   - Format tangkapan: 48kHz, 16-bit stereo PCM, dikompresi realtime menggunakan Opus codec (bitrate 32-64 kbps).
4. **Input Injection:**
   - Win32 API `SendInput` untuk simulasi mouse absolute coordinates (`MOUSEEVENTF_ABSOLUTE`), scroll delta, mouse click (left/right/middle), dan event keyboard (`KEYEVENTF_SCANCODE` untuk kompatibilitas game & shortcut Windows).

### 3.2. Android Subsystem (Mobile Host & Client)
1. **Screen Capture & Encoding:**
   - **Standalone Mode:**
     - Foreground Service dengan izin `MediaProjectionManager`.
     - Mengarahkan `VirtualDisplay` langsung ke `Surface` input dari `MediaCodec` (H.264 hardware encoder). Zero-copy buffer antar hardware encoder dan display subsystem.
   - **ADB Mode:**
     - Mendeteksi koneksi ADB lokal (`127.0.0.1:5555` atau via USB socket).
     - Menggunakan binary native injection langsung membaca `SurfaceControl` internal Android (serupa engine `scrcpy`), membuka kapabilitas frame rate tinggi (90Hz / 120Hz).
2. **Audio Capture:**
   - Menggunakan `AudioPlaybackCaptureConfiguration` (Android 10+) untuk menangkap audio internal aplikasi sistem secara jernih tanpa melibatkan mikrofon fisik.
   - Fallback audio: Jika OS di bawah Android 10, peringatan visual ditampilkan dan streaming berjalan video-only.
3. **Dual Input Injection System:**
   - **Mode A (Standalone - AccessibilityService):**
     - Memanfaatkan `AccessibilityService.dispatchGesture()` untuk mengeksekusi klik, sentuhan ganda, tap-and-drag, serta swipe multi-point.
     - Menggunakan `performGlobalAction()` untuk navigasi sistem: `GLOBAL_ACTION_BACK`, `GLOBAL_ACTION_HOME`, `GLOBAL_ACTION_RECENTS`.
   - **Mode B (High-Performance ADB Mode):**
     - Mengirim event input langsung melalui command line input injection atau socket uinput driver (`/dev/uinput`), menghilangkan overhead latency gesture engine.

---

## 4. Networking, Discovery & Security Architecture

### 4.1. Local Area Network (LAN) Discovery
- **Protokol:** UDP Multicast pada address `239.255.42.99:53210` (dan fallback UDP Broadcast ke subnet 255).
- **Discovery Payload:**
  ```json
  {
    "device_id": "849201",
    "name": "Laptop-Rafi",
    "os": "windows",
    "pubkey": "<base64_ed25519_public_key>",
    "port": 53211,
    "timestamp": 1726740000
  }
  ```
- **Koneksi LAN Langsung:**
  - Begitu perangkat dipilih dari daftar LAN, inisiasi koneksi langsung via TCP/UDP Socket lokal terenkripsi (tanpa bergantung pada STUN/TURN maupun internet).

### 4.2. Remote P2P & NAT Traversal (Internet)
- **WebRTC Data & Media Transport:**
  - Menggunakan WebRTC ICE framework dengan public STUN servers (e.g. `stun.l.google.com:19302`, `stun.cloudflare.com:3478`).
  - Mendukung Direct P2P Hole Punching untuk Full Cone, Restricted Cone, dan Port Restricted Cone NAT.
- **Stateless Signaling Protocol:**
  - Pertukaran SDP Offer/Answer dan ICE Candidates via lightweight websocket rendezvous worker (stateless, hanya mencocokkan hash Device ID tujuan dan langsung membuang sesi setelah handshake selesai).
- **Relay Fallback:**
  - Turn relay fallback ringan otomatis aktif jika kedua perangkat berada di balik Symmetric NAT (firewall ketat).

### 4.3. Security & End-to-End Encryption (E2EE)
- **Identitas Kriptografis:**
  - Setiap instalasi aplikasi men-generate pasangan kunci Ed25519 (identitas) dan X25519 (key exchange).
  - Device ID (6 digit angka) diturunkan dari truncated SHA-256 hash dari Public Key perangkat.
- **Pairing Handshake:**
  - **Dynamic One-Time PIN:** 6 digit PIN numerik di-generate dengan masa berlaku 120 detik. PIN digunakan sebagai Pre-Shared Key (PSK) dalam Diffie-Hellman Key Exchange (SPAKE2 / HKDF).
  - **QR Code Pairing:** Host menampilkan QR Code berisi `{ id, pubkey, nonce, session_token }`. Client melakukan scan untuk verifikasi seketika tanpa input manual.
- **Kanal Data & Media Terenkripsi:**
  - Seluruh media track (video/audio) dienkripsi dengan **DTLS-SRTP**.
  - Seluruh kontrol data (mouse, touch gesture, keyboard, clipboard) dienkripsi menggunakan **ChaCha20-Poly1305 AEAD**.

---

## 5. UI/UX Specification (Flutter)

### 5.1. Design Tokens & Styling
- Mengikuti estetika modern, clean, minimalis, dan hemat daya (Dark Mode default untuk layar AMOLED Android & laptop).
- Font: Inter / Roboto / System Font.
- Responsive layout yang otomatis beradaptasi antara orientasi Portrait HP, Landscape HP, dan Windowed/Fullscreen Desktop PC.

### 5.2. Screen Flows
1. **Home Screen:**
   - Status bar: Badge status (`Online (P2P)`, `LAN Ready`, `Connecting...`).
   - Kartu Device: ID Perangkat (`849 201`), tombol Copy, tombol Share QR Code, Dynamic PIN display + refresh button.
   - Quick Connect: Input `Target Device ID` + Tombol `Hubungkan`.
   - LAN Devices List: Menampilkan list card perangkat di Wi-Fi yang sama lengkap dengan icon OS (Windows/Android) dan tombol `Connect` 1-tap.
2. **Session Viewer (Remote Screen):**
   - Fullscreen GPU Texture Canvas (`Texture(textureId)`).
   - Minimalist Overlay HUD (Floating Toolbar):
     - Collapsible pill di sisi atas/samping.
     - Live Diagnostic info: Latency (`18 ms`), FPS (`60 fps`), Bitrate (`3.2 Mbps`), Packet Loss (`0.0%`).
     - Action Buttons:
       - Audio Toggle (Mute/Unmute remote sound).
       - Input Switcher (Trackpad mode vs Direct Touch mode saat kontrol PC dari Android).
       - Navigation Bar (Back, Home, Recents saat kontrol Android).
       - Desktop Helpers (Ctrl+Alt+Del, WinKey, TaskMgr saat kontrol PC).
       - Quality Switcher (Eco 720p 30fps, Balanced 1080p 60fps, Ultra).
       - End Session (Tombol merah konfirmasi cepat).
3. **Android Permission Setup Modal:**
   - 3-Step interactive onboarding:
     1. Aktifkan Izin Rekam Layar (MediaProjection prompt).
     2. Aktifkan Accessibility Service (Direct link ke menu Aksesibilitas Android).
     3. Mode ADB (Deteksi otomatis status USB/Wireless debugging dengan petunjuk singkat).

---

## 6. Performance Benchmarks & Targets

| Metric | Target (LAN Wi-Fi 5GHz) | Target (Internet P2P) | Ambang Batas Maksimal |
|---|---|---|---|
| **End-to-End Latency** | **15 - 28 ms** | **35 - 60 ms** | < 80 ms |
| **Frame Rate** | **60 FPS** (90/120 FPS ADB) | **60 FPS** (adaptif) | min 30 FPS |
| **PC CPU Usage** | **< 6%** (DXGI + HW MFT) | **< 8%** | < 15% pada i3 / Celeron |
| **PC RAM Footprint** | **< 90 MB** | **< 120 MB** | < 180 MB |
| **Android Battery Impact** | **~4-6% per jam streaming** | **~6-8% per jam** | < 10% per jam |
| **Encoding Overhead** | **< 5 ms per frame** | **< 8 ms per frame** | < 12 ms |

---

## 7. Error Handling & Recovery Strategies

1. **Jaringan Terputus / Wi-Fi Glitch:**
   - Buffer timeout 3.0 detik. Jika paket media terhenti, UI menampilkan overlay transparan: *"Mencoba menghubungkan kembali..."* dengan exponential backoff (1s, 2s, 4s).
   - Auto-ICE Restart tanpa mematikan sesi atau merusak state aplikasi.
2. **GPU Video Decoder Crash / Reset:**
   - Fallback otomatis ke software decoding frame demi frame tanpa disconnect sesi.
3. **Android Background Process Kill Guard:**
   - Menjalankan Foreground Service dengan persistent silent notification untuk mencegah Android LMK (Low Memory Killer) mematikan service capture saat streaming.

---

## 8. Git Repository Structure

```
mirror-remote/
├── Cargo.toml                    # Rust Workspace
├── rust_core/                    # Core Shared Engine
│   ├── Cargo.toml
│   └── src/
│       ├── api.rs                # FFI API for flutter_rust_bridge
│       ├── capture/
│       │   ├── windows_dxgi.rs   # DXGI Desktop Duplication
│       │   └── android_bridge.rs # JNI/NDK hooks
│       ├── encoder/
│       │   ├── mft_h264.rs       # Windows Media Foundation
│       │   └── openh264_sw.rs    # Software fallback
│       ├── audio/
│       │   └── wasapi.rs         # WASAPI Loopback capture
│       ├── input/
│       │   └── win_input.rs      # Win32 SendInput
│       ├── network/
│       │   ├── lan_discovery.rs  # UDP Multicast / Broadcast
│       │   ├── webrtc_transport.rs
│       │   └── crypto.rs         # E2EE & Handshake
│       └── lib.rs
├── flutter_app/                  # Cross-Platform Flutter App
│   ├── pubspec.yaml
│   ├── lib/
│   │   ├── main.dart
│   │   ├── bridge_generated/     # FRB bindings
│   │   ├── views/
│   │   │   ├── home_view.dart
│   │   │   ├── session_viewer_view.dart
│   │   │   └── permissions_view.dart
│   │   ├── controllers/
│   │   │   ├── session_controller.dart
│   │   │   └── discovery_controller.dart
│   │   └── widgets/
│   │       ├── lan_device_card.dart
│   │       └── remote_toolbar.dart
│   ├── android/
│   │   └── app/src/main/kotlin/.../
│   │       ├── CaptureService.kt # MediaProjection & MediaCodec
│   │       └── InputAccessibilityService.kt
│   └── windows/
│       └── runner/
└── docs/
    └── superpowers/specs/
        └── 2026-09-19-screen-mirror-remote-control-design.md
```
