# Task 1 Brief: Rust Opus Audio Encoder & VAUD Format 2 Packetization

## Objective
Implement real-time Opus audio encoding in the Rust host (`rust/src/audio.rs`), packaging 48kHz stereo WASAPI loopback capture into compact 20ms Opus frames inside binary `VAUD` packets with format byte `0x02`.

## Context & Files
- Working Directory: `D:/Software/Hermes Workspace/projects/vrv-desk/rust`
- Key Files:
  - `rust/src/audio.rs`: Current WASAPI capturer and `VAUD` packet header.
  - `rust/src/bin/vrv_host.rs`: Host streaming daemon.
  - `rust/Cargo.toml`: Already has `opus = "0.4.0"` included.
  - Test file to create: `rust/tests/audio_opus_test.rs`.

## Requirements
1. **Audio Constants**:
   - `AUDIO_FORMAT_PCM: u8 = 0x01`
   - `AUDIO_FORMAT_OPUS: u8 = 0x02`
2. **`OpusAudioEncoder`**:
   - Create struct `OpusAudioEncoder` in `rust/src/audio.rs`:
     - Configurable sample rate (default 48000), channels (2), bitrate (default 64,000 bps).
     - 20ms frame buffer = 960 samples per channel (1920 `i16` values = 3840 bytes).
     - `feed_pcm_and_encode(&mut self, pcm_bytes: &[u8]) -> Result<Vec<Vec<u8>>, String>`:
       - Converts `i16` little-endian bytes to samples.
       - Loops while enough samples exist for a full 20ms frame.
       - Calls `opus::Encoder::encode` into output buffer.
       - Wraps encoded bytes in 8-byte `VAUD` header with `format = AUDIO_FORMAT_OPUS` (0x02).
       - Returns list of complete binary `VAUD` packets ready for transmission.
3. **Integration with `WasapiAudioCapturer` / `AudioCapturer`**:
   - Add optional `use_opus: bool` (default true) or wrap capturer so `get_chunks()` emits Opus `VAUD` packets.
   - If Opus encoder fails to initialize, gracefully fall back to raw PCM (`format = 0x01`).
4. **Unit and Integration Tests**:
   - Write `rust/tests/audio_opus_test.rs` testing:
     - Initialization of `OpusAudioEncoder`.
     - Feeding sine wave PCM and verifying encoded `VAUD` packet header (`VAUD`, format 0x02, channels 2, rate 48000).
     - Packet size verification (< 300 bytes per 20ms frame vs 3840 bytes PCM, >10x compression).
     - Full audio capturer chunk emission.
5. **Compilation & Verification**:
   - Run `cargo check` and `cargo test --test audio_opus_test`.
   - Run all test targets (`cargo test`).
   - Recompile `vrv_host` release binary and install to `dist/windows/vrv_host.exe`.
   - Write completion report to `.superpowers/sdd/2026-09-20-opus-audio-compression/task-1-report.md`.
