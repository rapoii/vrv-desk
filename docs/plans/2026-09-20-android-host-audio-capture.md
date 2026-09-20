# Android Host Audio Capture (AudioPlaybackCaptureConfiguration) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement native internal audio capture on Android 10+ using `AudioPlaybackCaptureConfiguration`, package PCM data into 8-byte binary `VAUD` packets, multiplex over the E2EE WebSocket connection, and display fallback warnings for OS versions below Android 10.

**Architecture:** `MediaProjectionService.kt` captures PCM audio via `AudioRecord` and `AudioPlaybackCaptureConfiguration`, encapsulates it with standard `VAUD` magic framing, and forwards it to `AndroidHostService.dart` via `EventChannel`. Connected PC and mobile clients demux `VAUD` packets alongside `VH24` video.

**Tech Stack:** Kotlin (Android 10+ AudioPlaybackCapture, AudioRecord, MediaProjection), Dart/Flutter (MethodChannel, EventChannel, E2EE transport), JUnit/Flutter test.

**Spec Reference:** `docs/ARCHITECTURE.md` Section 3.2 Item 2.

---

### Task 1: Flutter Service & UI Layer (Audio Support Detection & Dialog Warning)

**Files:**
- Modify: `lib/src/services/android_host_service.dart`
- Modify: `lib/src/widgets/host_mode_dialog.dart`
- Modify: `test/android_host_service_test.dart`
- Modify: `test/host_mode_dialog_test.dart`

- [x] **Step 1: Write unit and widget tests for audio capability detection and UI fallback badge**
- [x] **Step 2: Run tests to verify failure**
- [x] **Step 3: Implement `isInternalAudioSupported` in `AndroidHostService` and UI fallback in `HostModeDialog`**
- [x] **Step 4: Run tests to verify they pass**
- [x] **Step 5: Commit**

---

### Task 2: Native Android Audio Capture Implementation (`MediaProjectionService.kt` & `MainActivity.kt`)

**Files:**
- Modify: `android/app/src/main/kotlin/com/vrvdesk/app/MediaProjectionService.kt`
- Modify: `android/app/src/main/kotlin/com/vrvdesk/app/MainActivity.kt`

- [x] **Step 1: Implement `AudioPlaybackCaptureConfiguration` and `AudioRecord` loop in `MediaProjectionService.kt`**
- [x] **Step 2: Package PCM audio chunks into binary `VAUD` packets (8-byte header: `VAUD`, format 0x01, channels 2, sample_rate 48000)**
- [x] **Step 3: Handle graceful startup, error fallback on pre-Android 10, and cleanup in `stopAudioCapture`**
- [x] **Step 4: Wire `isInternalAudioSupported` and `enable_audio` in `MainActivity.kt`**
- [x] **Step 5: Verify build with `./gradlew assembleDebug` or `flutter build apk`**
- [x] **Step 6: Commit**

---

### Task 3: Multiplexed E2EE Audio & Video Streaming Verification

**Files:**
- Modify: `test/android_host_service_test.dart`

- [x] **Step 1: Add test verifying multiplexed VAUD + VH24 transmission over encrypted E2EE WebSocket**
- [x] **Step 2: Verify all Flutter tests pass (`flutter test`)**
- [x] **Step 3: Commit and push**
