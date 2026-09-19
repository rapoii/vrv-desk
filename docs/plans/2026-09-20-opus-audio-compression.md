# Phase 9 Implementation Plan: Real-time Opus Audio Compression (Pilihan B: Tahap 1)

## Overview
Currently, VrV Desk captures audio via Windows WASAPI Loopback as raw, uncompressed 16-bit 48kHz stereo PCM transmitted via binary `VAUD` packets over WebSocket. While low latency, raw PCM consumes **~192 KB/s (1.54 Mbps)** of bandwidth constantly.
This phase implements **real-time Opus audio compression** at 64 kbps, dropping audio streaming bandwidth down to **~8 KB/s** (a **95.8% bandwidth reduction** / ~14x compression ratio), while maintaining pristine audio fidelity and sub-20ms latency.

## Protocol Specification (`VAUD` Packet v2)
The binary `VAUD` packet header structure is preserved and extended via byte 4 (`format`):
- `[0..4]`: ASCII `b"VAUD"` (Magic Identifier, 4 bytes)
- `[4..5]`: Format Tag (1 byte):
  - `0x01`: `PCM_S16LE` (Uncompressed 16-bit PCM raw fallback)
  - `0x02`: `OPUS` (Opus-encoded frame at 48kHz stereo)
- `[5..6]`: Channels (1 byte): `2` (Stereo)
- `[6..8]`: Sample Rate (2 bytes, u16 little-endian): `48000`
- `[8..N]`: Payload:
  - If `0x01`: Raw PCM bytes (3840 bytes per 20ms chunk)
  - If `0x02`: Opus compressed frame (~80 - 250 bytes per 20ms chunk)

## Architecture Components
1. **Rust Host (`rust/src/audio.rs` & `rust/src/bin/vrv_host.rs`)**:
   - `OpusAudioEncoder`: Accumulates 20ms of audio (960 stereo samples = 1920 `i16` values = 3840 bytes).
   - Encodes via `opus::Encoder::new(48000, Channels::Stereo, Application::Audio)` at 64 kbps bitrate.
   - Prepends the 8-byte `VAUD` header with `format = 0x02`.
   - Sends the compressed frame down the WebSocket streaming loop.
2. **Flutter Service (`lib/src/services/audio_stream_player.dart`)**:
   - Recognizes `AudioFormatTag.opus = 0x02`.
   - Forwards format tag alongside binary payload to the native Android platform channel.
3. **Android Client (`com.vrvdesk.app`)**:
   - `OpusAudioDecoder.kt`: Wraps `android.media.MediaCodec` configured for `audio/opus` (MIME type `audio/opus`) with standard `csd-0`, `csd-1`, and `csd-2` headers.
   - Decompresses Opus packets into PCM frames on a low-latency thread.
   - Feeds PCM frames directly into `AudioTrack` with real-time chunk metrics.
   - Preserves raw PCM `0x01` fallback execution for complete backwards compatibility.

## Tasks
- **Task 1**: Implement Rust Opus Audio Encoder & VAUD Format 2 Packetization (`rust/src/audio.rs`, `rust/tests/audio_opus_test.rs`).
- **Task 2**: Implement Flutter Audio Format Demux & Android MediaCodec Opus Decoder (`audio_stream_player.dart`, `MainActivity.kt`, `OpusAudioDecoder.kt`).
- **Task 3**: End-to-End Verification, Latency & Bandwidth Benchmark, Emulator Verification, and Release v0.9.0.
