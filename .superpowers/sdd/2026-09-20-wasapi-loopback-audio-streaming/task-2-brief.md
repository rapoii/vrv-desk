# Task 2 Brief: Android Native AudioTrack & Flutter Audio Stream Player

## Working Directory
`projects/vrv-desk`

## Goal
Implement low-latency PCM audio playback on Android via native `AudioTrack` and integrate the audio player and UI mute/volume controls into Flutter (`MainActivity.kt`, `audio_stream_player.dart`, `mirror_view.dart`, `dock_bar.dart`).

## Requirements
1. **Native Android AudioTrack (`android/app/src/main/kotlin/com/mirror/app/mirror_app/MainActivity.kt`)**:
   - Register `MethodChannel("com.vrv.desk/audio")` in `configureFlutterEngine`.
   - Methods:
     - `init(sampleRate: Int, channels: Int)`: Creates and starts `AudioTrack` in `MODE_STREAM` (16-bit PCM, stereo/mono, `AudioAttributes.USAGE_MEDIA`).
     - `write(data: ByteArray)`: Writes raw PCM bytes using `audioTrack.write(data, 0, data.size, AudioTrack.WRITE_NON_BLOCKING)`.
     - `setMuted(muted: Boolean)`: Toggles volume (`0.0f` vs `1.0f`).
     - `stop()`: Pauses, flushes, and releases `AudioTrack`.
   - Graceful error handling (if AudioTrack fails to initialize, don't crash).

2. **Dart Service (`lib/src/services/audio_stream_player.dart`)**:
   - Create `AudioStreamPlayer`:
     - Handles communication with `com.vrv.desk/audio`.
     - Automatically handles initialization when sample rate or channels change.
     - `feedPacket(int format, int channels, int sampleRate, Uint8List pcm)`: Buffers/delivers PCM chunks to the native channel.
     - `setMuted(bool muted)`
     - `stop()` / `dispose()`
     - Fallback for non-Android platforms (no-op so unit tests and desktop builds pass without errors).

3. **Demultiplexing in `lib/src/views/mirror_view.dart`**:
   - In WebSocket binary listener:
     - Check magic bytes:
       - `data[0] == 0xFF && data[1] == 0xD8` -> Video JPEG frame.
       - `data[0] == 0x56 && data[1] == 0x41 && data[2] == 0x55 && data[3] == 0x44` (`VAUD`) -> Audio packet.
   - On `auth_ok`, ensure `AudioStreamPlayer` is active.
   - On session end / `dispose`, stop `AudioStreamPlayer`.

4. **UI Audio Toggle in Floating Dock (`lib/src/views/mirror_view.dart` or dock control)**:
   - Add Audio / Speaker icon to the floating utility dock:
     - Icon: `_isAudioMuted ? Icons.volume_off : Icons.volume_up`
     - Tapping toggles mute state with feedback.

5. **Unit & Widget Tests (`test/audio_stream_test.dart`)**:
   - Mock the method channel `com.vrv.desk/audio`.
   - Verify `AudioStreamPlayer` packet handling, sample rate parsing, mute toggling, and stop.
   - Ensure `flutter test` and `flutter analyze` pass with 0 issues.

6. **Report**:
   - Write report to `.superpowers/sdd/2026-09-20-wasapi-loopback-audio-streaming/task-2-report.md`.
