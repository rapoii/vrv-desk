# Implementation Plan: Real-Time Audio Streaming (WASAPI Loopback Capture)

## Goal
Enable real-time audio streaming from the host Windows PC to the Android client in VrV Desk (`rapoii/vrv-desk`). When the host plays audio (YouTube, games, media, system sounds), the audio is captured via Windows WASAPI Loopback, framed as binary audio packets, transmitted over WebSocket alongside video frames, and played with ultra-low latency on the Android client using native Android `AudioTrack`.

## Architecture & Design

### 1. Host (Rust Windows Audio Loopback Capture: `rust/src/audio.rs`)
- Uses Windows WASAPI via `windows` crate (`Win32_Media_Audio`, `Win32_System_Com`).
- Enumerates default render endpoint (`eRender`, `eConsole`) and activates `IAudioClient` with `AUDCLNT_STREAMFLAGS_LOOPBACK`.
- Reads mix format (`WAVEFORMATEX` / `WAVEFORMATEXTENSIBLE`).
- Normalizes incoming audio samples (32-bit float or 16-bit PCM) into standard **16-bit signed PCM (S16LE)** at 48,000 Hz stereo (or native sample rate with header tags).
- Packages audio into binary packets with a 8-byte header:
  - Byte 0..4: Magic bytes `VAUD` (`0x56, 0x41, 0x55, 0x44`)
  - Byte 4: Format tag (`0x01` = PCM 16-bit LE)
  - Byte 5: Channels (`0x02` = Stereo, `0x01` = Mono)
  - Byte 6..8: Sample rate in kHz as u16 LE (e.g. `48000`)
  - Byte 8..: Raw PCM S16LE byte buffer.
- Emits chunks of 10-20ms (~960 to 1920 frames = 3,840 to 7,680 bytes).
- Background capture loop feeds into a bounded channel or sends directly through the session's WebSocket writer.

### 2. Protocol & Demultiplexing
- On WebSocket receiver in Flutter:
  - If binary data begins with `0xFF, 0xD8` -> Video JPEG Frame (update `_currentFrame`).
  - If binary data begins with `0x56, 0x41, 0x55, 0x44` (`VAUD`) -> Audio Packet (parse format, sample rate, channels, pass PCM payload to audio player).
  - If text data -> JSON control message (`auth_required`, `clipboard_sync`, etc.).
- Audio packets are ONLY transmitted while session state is `authenticated` (`auth_ok`).

### 3. Android Client (Native `AudioTrack` via Platform Channel: `MainActivity.kt` & `audio_player.dart`)
- Implement low-latency PCM streaming in `android/app/src/main/kotlin/com/mirror/app/mirror_app/MainActivity.kt`:
  - `MethodChannel("com.vrv.desk/audio")`
  - Methods:
    - `init(sampleRate: Int, channels: Int)`: Configures and starts `AudioTrack` in `MODE_STREAM` with `AudioAttributes.USAGE_MEDIA` and `AudioAttributes.CONTENT_TYPE_UNKNOWN`.
    - `write(pcmData: ByteArray)`: Direct buffer write to `audioTrack.write()`.
    - `setMuted(muted: Boolean)`: Toggles volume / playback without disconnecting.
    - `stop()`: Pauses, flushes, and releases `AudioTrack`.
- Dart wrapper: `lib/src/services/audio_stream_player.dart`.
- In `MirrorView`:
  - Initialize audio player on `auth_ok`.
  - Add speaker / mute toggle button to the floating dock (`Icons.volume_up` / `Icons.volume_off`).
  - Handle audio chunks in `ws.listen`.
  - Release `AudioTrack` on dispose.

## Tasks Breakdown
- [ ] Task 1: Rust Windows WASAPI Loopback Capture & Binary Packet Framing (`rust/src/audio.rs`, `rust/src/bin/vrv_host.rs`, `rust/tests/audio_test.rs`).
- [ ] Task 2: Android Native `AudioTrack` & Flutter Audio Stream Player (`MainActivity.kt`, `audio_stream_player.dart`, `mirror_view.dart`, `dock_bar.dart`).
- [ ] Task 3: Integration & End-to-End Verification (Synthetic and live PC audio capture test, unit tests, APK build, and live test on emulator).
