# High-Performance ADB Mode & ADB Input Bridge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`...[truncated]

**Goal:** Implement High-Performance ADB Mode and `AdbInputBridge.kt` to allow low-latency input injection directly via shell/uinput (sub-10ms bypass of Accessibility gestures), detect local ADB connectivity (127.0.0.1:5555), and support high refresh rate displays (90Hz / 120Hz).

**Architecture:**
- `AdbInputBridge.kt`: Manages persistent low-latency input stream (`input tap`, `input swipe`, `input keyevent`, `/dev/uinput`), checks ADB daemon port (127.0.0.1:5555) and debugging status.
- `MainActivity.kt`: Exposes `com.vrv.desk/adb_bridge` MethodChannel to query ADB state, refresh rate, and route inputs.
- `AndroidHostService.dart`: Automatically routes remote touch/key events through ADB Bridge when available, with automatic fallback to AccessibilityService.
- `HostModeDialog.dart`: Renders ADB High-Performance badge and refresh rate capabilities (60Hz / 90Hz / 120Hz).

**Spec Reference:** `docs/ARCHITECTURE.md` Section 3.2 Items 1 & 3; `docs/IMPLEMENTATION_PLAN.md` Line 83; `README.md` Item 16.

---

### Task 1: Native `AdbInputBridge.kt` & Native MethodChannel Bridge

**Files:**
- Create: `android/app/src/main/kotlin/com/vrvdesk/app/AdbInputBridge.kt`
- Modify: `android/app/src/main/kotlin/com/vrvdesk/app/MainActivity.kt`

- [x] **Step 1: Implement `AdbInputBridge.kt` with persistent shell stream, port 5555 probe, and refresh rate query**
- [x] **Step 2: Connect `AdbInputBridge` to MethodChannel in `MainActivity.kt`**
- [x] **Step 3: Verify Kotlin compilation with `flutter build apk --debug --target-platform android-arm64`**
- [x] **Step 4: Commit**

---

### Task 2: Flutter Service Layer Integration (`AndroidHostService.dart`)

**Files:**
- Modify: `lib/src/services/android_host_service.dart`
- Modify: `test/android_host_service_test.dart`

- [x] **Step 1: Write unit tests in `android_host_service_test.dart` for ADB status probe and input routing**
- [x] **Step 2: Run tests to verify failure**
- [x] **Step 3: Implement ADB status query, high refresh rate mode, and dual input routing in `AndroidHostService.dart`**
- [x] **Step 4: Run tests to verify they pass**
- [x] **Step 5: Commit**

---

### Task 3: UI Integration (`HostModeDialog.dart`) & End-to-End Verification

**Files:**
- Modify: `lib/src/widgets/host_mode_dialog.dart`
- Modify: `test/host_mode_dialog_test.dart`

- [x] **Step 1: Write widget tests for ADB Mode badge and refresh rate selector in `host_mode_dialog_test.dart`**
- [x] **Step 2: Run widget tests to verify failure**
- [x] **Step 3: Implement UI elements in `HostModeDialog.dart`**
- [x] **Step 4: Run full test suite (`flutter test` and `cargo test`)**
- [x] **Step 5: Commit and push**
