# Android Host Screen Capture & Remote Input (Phase 14) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Menjadikan Android sebagai Host yang mampu merekam layar via MediaProjection + MediaCodec hardware H.264 (`VH24`), melayani koneksi streaming E2EE ChaCha20-Poly1305, menyiarkan beacon LAN UDP (:53210), serta menerima injeksi kontrol remote dari PC melalui Android `AccessibilityService`.

**Architecture:** 
- Native Kotlin: `MediaProjectionService.kt` (Foreground Service rekam layar GPU ke MediaCodec Surface encoder `video/avc`) dan `InputAccessibilityService.kt` (injeksi gesture klik, drag, back, home).
- Dart / Flutter Host Layer: `AndroidHostService` mengelola embedded WebSocket server (:53211), autentikasi One-Time PIN, enkripsi ChaCha20-Poly1305, serta penyiaran UDP discovery beacon.
- Presentation: `HostModeDialog` / `PermissionModal` untuk panduan aktivasi izin MediaProjection & Accessibility.

**Tech Stack:** Kotlin, Android MediaProjection API, MediaCodec H.264 hardware encoder, Android AccessibilityService, Flutter/Dart `dart:io` WebSocket & UDP Socket, `cryptography` ChaCha20-Poly1305.

**Spec:** `docs/ARCHITECTURE.md` (Bab 1 baris 14, Bab 3.2 baris 69–86, Bab 5.2.3 baris 155–160).

## Global Constraints
- Android SDK Min API: 26 (Android 8.0), Target API: 34 (Android 14).
- Binary Wire Format: Video tetap dibungkus header 12-byte `b"VH24"` yang identik dengan pipeline Windows host agar kompatibel dengan decoder Flutter/PC.
- Keamanan: Seluruh frame video dan input kontrol wajib dienkripsi simetris AEAD ChaCha20-Poly1305 (`VE2E` framing 28-byte overhead).
- Zero Crash: Foreground service harus memiliki persistent notification channel agar tidak dimatikan Android Low Memory Killer (LMK).

---

### Task 1: Android Manifest, Permissions, & Accessibility Configuration

**Files:**
- Create: `android/app/src/main/res/xml/accessibility_service_config.xml`
- Modify: `android/app/src/main/AndroidManifest.xml`

**Interfaces:**
- Produces: Deklarasi izin `FOREGROUND_SERVICE`, `FOREGROUND_SERVICE_MEDIA_PROJECTION`, service `MediaProjectionService`, dan `InputAccessibilityService`.

- [ ] **Step 1: Buat file konfigurasi Accessibility XML**
  Buat `android/app/src/main/res/xml/accessibility_service_config.xml` dengan atribut `canPerformGestures="true"` dan `accessibilityFeedbackType="feedbackGeneric"`.

- [ ] **Step 2: Update AndroidManifest.xml**
  Tambahkan izin rekam layar foreground service dan registrasi service `MediaProjectionService` dan `InputAccessibilityService`.

- [ ] **Step 3: Verifikasi build Android manifest**
  Jalankan `flutter build apk --debug` untuk memvalidasi manifest merging tanpa error.

- [ ] **Step 4: Commit konfigurasi permission**
  `git add android/app/src/main/AndroidManifest.xml android/app/src/main/res/xml/accessibility_service_config.xml && git commit -m "feat(android): configure manifest permissions for MediaProjection and AccessibilityService"`

---

### Task 2: Native Android MediaProjection & MediaCodec H.264 Encoder

**Files:**
- Create: `android/app/src/main/kotlin/com/vrvdesk/app/MediaProjectionService.kt`
- Modify: `android/app/src/main/kotlin/com/vrvdesk/app/MainActivity.kt`

**Interfaces:**
- Produces: MethodChannel `com.vrv.desk/capture` dengan fungsi `startCapture(resultCode, intentData, width, height, bitrate, fps)` dan `stopCapture()`, serta EventChannel `com.vrv.desk/capture_stream` yang memancarkan byte array paket `VH24`.

- [ ] **Step 1: Implementasikan MediaProjectionService.kt**
  - Buat Foreground Service dengan Notification Channel `vrv_desk_capture`.
  - Inisiasi `MediaCodec.createEncoderByType("video/avc")`.
  - Setup input `Surface` dari encoder dan tautkan ke `mediaProjection.createVirtualDisplay`.
  - Drain encoder output buffers di background thread loop, ekstrak Annex B NAL units (SPS/PPS/IDR/P-frame).
  - Format output dengan header 12-byte `b"VH24"`:
    `[0..4] b"VH24"` + `[4..8] length` + `[8] flags (0x01 if keyframe)` + `[9..12] sequence`.
  - Callback ke EventChannel / Flutter sink.

- [ ] **Step 2: Tautkan Capture MethodChannel di MainActivity.kt**
  - Tambahkan `startCaptureIntent` untuk memanggil `mediaProjectionManager.createScreenCaptureIntent()`.
  - Tangani `onActivityResult` (request code `MEDIA_PROJECTION_REQUEST_CODE`), kirim intent token ke `MediaProjectionService`.
  - Siapkan `EventChannel` untuk streaming frame video `VH24` ke Dart.

- [ ] **Step 3: Uji kompilasi kode Kotlin**
  Jalankan `./gradlew compileDebugKotlin` di direktori `android/` untuk memastikan tidak ada syntax/type mismatch.

- [ ] **Step 4: Commit MediaProjectionService**
  `git add android/app/src/main/kotlin/com/vrvdesk/app/MediaProjectionService.kt android/app/src/main/kotlin/com/vrvdesk/app/MainActivity.kt && git commit -m "feat(android): implement native MediaProjection and MediaCodec H.264 encoder"`

---

### Task 3: Native Android Input Accessibility Service

**Files:**
- Create: `android/app/src/main/kotlin/com/vrvdesk/app/InputAccessibilityService.kt`
- Modify: `android/app/src/main/kotlin/com/vrvdesk/app/MainActivity.kt`

**Interfaces:**
- Produces: MethodChannel `com.vrv.desk/accessibility` dengan method `isAccessibilityEnabled()`, `openAccessibilitySettings()`, `tap(x, y)`, `swipe(x1, y1, x2, y2, duration)`, dan `globalAction(actionName)`.

- [ ] **Step 1: Implementasikan InputAccessibilityService.kt**
  - Simpan instance aktif di static `sharedInstance`.
  - Fungsi `tap(x, y)`: buat `GestureDescription` dengan `Path().apply { moveTo(x, y); lineTo(x, y) }` durasi 40ms, panggil `dispatchGesture()`.
  - Fungsi `swipe(x1, y1, x2, y2, duration)`: buat gesture path dari (x1, y1) ke (x2, y2).
  - Fungsi `performGlobalAction(action)`: petakan `back` -> `GLOBAL_ACTION_BACK`, `home` -> `GLOBAL_ACTION_HOME`, `recents` -> `GLOBAL_ACTION_RECENTS`.

- [ ] **Step 2: Tautkan Accessibility MethodChannel di MainActivity.kt**
  - Buat handler channel `com.vrv.desk/accessibility` untuk memanggil method di `InputAccessibilityService`.
  - Tambahkan pemeriksaan status apakah accessibility service sedang aktif di Android settings.

- [ ] **Step 3: Uji kompilasi Kotlin**
  Jalankan `./gradlew compileDebugKotlin` untuk memvalidasi integrasi service.

- [ ] **Step 4: Commit InputAccessibilityService**
  `git add android/app/src/main/kotlin/com/vrvdesk/app/InputAccessibilityService.kt android/app/src/main/kotlin/com/vrvdesk/app/MainActivity.kt && git commit -m "feat(android): implement InputAccessibilityService for remote gesture & navigation injection"`

---

### Task 4: Dart Android Host Streaming Controller (`AndroidHostService`)

**Files:**
- Create: `lib/src/services/android_host_service.dart`
- Test: `test/android_host_service_test.dart`

**Interfaces:**
- Consumes: `EventChannel('com.vrv.desk/capture_stream')` & `MethodChannel('com.vrv.desk/accessibility')`.
- Produces: `AndroidHostService` yang mengelola `HttpServer` (:53211), dynamic PIN 6-digit, sesi E2EE ChaCha20-Poly1305, enkripsi frame `VH24`, dan UDP Discovery Beacon (`:53210`).

- [ ] **Step 1: TDD Unit Test AndroidHostService**
  Tulis `test/android_host_service_test.dart` yang menguji lifecycle: start/stop server WebSocket, validasi PIN handshake, enkripsi paket data `VH24`, dan handling event input JSON.

- [ ] **Step 2: Jalankan test untuk memverifikasi failure**
  `flutter test test/android_host_service_test.dart` (Harus FAIL karena service belum ada).

- [ ] **Step 3: Implementasikan AndroidHostService**
  - Listen ke `EventChannel` video stream Kotlin.
  - Bind `HttpServer.bind(InternetAddress.anyIPv4, 53211)` dengan upgrade WebSocket.
  - Verifikasi PIN handshake (`auth_verify` / `auth_ok`).
  - Derivasi symmetric key E2EE via `E2eeTransportSession`.
  - Kirim ciphertext frame `VH24` ke seluruh client yang terautentikasi.
  - Teruskan event input mouse/touch ke `InputAccessibilityService`.
  - Jalankan timer UDP beacon `:53210` menyiarkan identitas Android Host.

- [ ] **Step 4: Jalankan unit test kembali**
  `flutter test test/android_host_service_test.dart` (Harus PASS 100%).

- [ ] **Step 5: Commit AndroidHostService**
  `git add lib/src/services/android_host_service.dart test/android_host_service_test.dart && git commit -m "feat: implement AndroidHostService for local WebSocket server, E2EE streaming, and discovery"`

---

### Task 5: UI Onboarding & Host Broadcasting Modal (`HostModeDialog`)

**Files:**
- Create: `lib/src/widgets/host_mode_dialog.dart`
- Modify: `lib/src/views/home_view.dart`
- Test: `test/host_mode_dialog_test.dart`

**Interfaces:**
- Produces: Widget `HostModeDialog` yang menampilkan tombol "Start Broadcasting", status permission check (MediaProjection & Accessibility), PIN display aktif, QR code sharing, dan tombol "Stop Broadcasting".

- [ ] **Step 1: TDD Widget Test HostModeDialog**
  Tulis `test/host_mode_dialog_test.dart` memverifikasi rendering status PIN, tombol toggle broadcast, dan dialog peringatan izin.

- [ ] **Step 2: Jalankan test untuk memverifikasi failure**
  `flutter test test/host_mode_dialog_test.dart` (Harus FAIL).

- [ ] **Step 3: Implementasikan HostModeDialog**
  - Tampilkan kartu status "Broadcast Screen (Host)".
  - Tambahkan tombol pintasan ke Settings Aksesibilitas Android jika belum diaktifkan.
  - Tampilkan counter frame yang dikirim dan status client yang tersambung.

- [ ] **Step 4: Integrasikan ke HomeView**
  Tambahkan tombol `Share My Screen` pada `HomeView` (khusus saat berjalan di Android) untuk membuka `HostModeDialog`.

- [ ] **Step 5: Jalankan unit test dan widget test**
  `flutter test test/host_mode_dialog_test.dart` (Harus PASS).

- [ ] **Step 6: Commit Host UI**
  `git add lib/src/widgets/host_mode_dialog.dart lib/src/views/home_view.dart test/host_mode_dialog_test.dart && git commit -m "feat(ui): add HostModeDialog and HomeView integration for Android screen sharing"`

---

### Task 6: Android Emulator Live Verification & Full Regression Suite

**Files:**
- Test artifacts: `emulator_host_dialog.png`, `emulator_permission_prompt.png`.

- [ ] **Step 1: Build Release Split APK**
  `flutter build apk --release --split-per-abi`
  Verifikasi ukuran APK tetap di bawah batas ~23 MB.

- [ ] **Step 2: Deploy & Validasi di Emulator-5554**
  Pasang APK x86_64 ke `emulator-5554` via `adb install -r -d`.
  Luncurkan app, buka `HostModeDialog`, verifikasi tombol izin dan UI layout via `vision_analyze`.

- [ ] **Step 3: Jalankan Full Regression Test Suite**
  - `cargo test`: Pastikan 45/45 Rust tests lulus.
  - `flutter test`: Pastikan seluruh unit & widget tests (termasuk modul baru) lulus 100%.

- [ ] **Step 4: Commit & Final Tag**
  Update dokumentasi, tag versi baru, dan laporkan bukti verifikasi ke pengguna.
