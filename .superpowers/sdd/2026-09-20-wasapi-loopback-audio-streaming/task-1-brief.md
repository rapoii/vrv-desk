# Task 1 Brief: Rust WASAPI Audio Loopback Capture & VAUD Packet Framing

## Working Directory
`projects/vrv-desk/rust`

## Goal
Implement real-time Windows audio loopback capture using WASAPI (`AUDCLNT_STREAMFLAGS_LOOPBACK`) in Rust, package PCM audio data into binary `VAUD` packets, and integrate audio streaming into `vrv_host.rs`.

## Requirements
1. **Module `rust/src/audio.rs`**:
   - Audio packet specification:
     - Byte 0..4: `b"VAUD"` (`0x56, 0x41, 0x55, 0x44`)
     - Byte 4: Format tag `0x01` (PCM S16LE)
     - Byte 5: Channels (e.g. `0x02` for stereo)
     - Byte 6..8: Sample rate as `u16` Little-Endian (e.g. `48000`)
     - Byte 8..: Raw PCM 16-bit signed LE bytes
   - Provide helper functions:
     - `pub fn encode_audio_packet(format: u8, channels: u8, sample_rate: u16, pcm: &[u8]) -> Vec<u8>`
     - `pub fn decode_audio_packet(packet: &[u8]) -> Option<AudioPacketHeader>`
   - Implement `AudioLoopbackCapturer`:
     - Under `#[cfg(windows)]`, use WASAPI COM interfaces from `windows` crate:
       `CoInitializeEx`, `MMDeviceEnumerator`, `eRender`, `eConsole`, `IAudioClient`, `IAudioCaptureClient`, `AUDCLNT_STREAMFLAGS_LOOPBACK`, `AUDCLNT_SHAREMODE_SHARED`.
     - Read mix format (`WAVEFORMATEX` / `WAVEFORMATEXTENSIBLE`).
     - Support 32-bit float (`WAVE_FORMAT_IEEE_FLOAT` / `KSDATAFORMAT_SUBTYPE_IEEE_FLOAT`) conversion to 16-bit integer PCM `i16`:
       `sample = (f32_sample.clamp(-1.0, 1.0) * 32767.0) as i16;`
     - Provide a method or channel receiver to read captured audio packets (e.g. `read_packet(&mut self) -> Option<Vec<u8>>` or background thread channel).
     - Under non-Windows or when no audio device is found, provide a mock/silent fallback so tests compile and run everywhere.
2. **Export in `rust/src/lib.rs`**:
   - Add `pub mod audio;`.
3. **Integration in `rust/src/bin/vrv_host.rs`**:
   - In `handle_streaming_session`, start the audio capturer and spawn an audio streaming task alongside the frame task and clipboard task.
   - When a packet is captured, send it via `ws_sender.send(Message::Binary(packet.into()))`.
4. **Test Suite `rust/tests/audio_test.rs`**:
   - Test packet encoding and decoding (header validation, magic bytes, payload integrity).
   - Test audio packet detection (verify `is_audio_packet` returns true for `VAUD` and false for JPEG `[0xFF, 0xD8]`).
   - Test loopback capturer initialization and fallback.
5. **Report**:
   - Save report to `.superpowers/sdd/2026-09-20-wasapi-loopback-audio-streaming/task-1-report.md`.
