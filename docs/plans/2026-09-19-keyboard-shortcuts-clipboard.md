# Virtual Keyboard, Shortcuts & Clipboard Synchronization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement virtual keyboard typing, Windows shortcut buttons (WinKey, TaskMgr, Alt+Tab, Esc, Enter), and bidirectional clipboard synchronization between Windows PC and Android devices.

**Architecture:** Extend the Rust WebSocket Host engine with Win32 `SendInput` Unicode character typing, shortcut combination synthesis, and Win32 clipboard monitoring. In Flutter `MirrorView`, add a floating control dock with soft-keyboard bridge, quick shortcut bar, and clipboard sync notifications.

**Tech Stack:** Rust (edition 2021, `windows` Win32 API for keyboard & clipboard), Flutter 3.x (Dart `Clipboard`, `FocusNode`, `RawKeyboardListener`/`TextField`).

**Spec:** `docs/ARCHITECTURE.md`

## Global Constraints
- Target platforms: Windows 10/11 (Host) and Android 8.0+ (Client).
- Zero external cloud services: Communication strictly over the established WebSocket session on port 53211.
- Latency budget: Keystrokes and shortcuts must execute on Windows within <10ms of Android input.
- Split APK per ABI: Android builds must keep split-per-abi (`arm64-v8a` < 25MB).

---

### Task 1: Rust Host Keyboard Typing, Shortcuts & Clipboard Support

**Files:**
- Modify: `projects/vrv-desk/rust/src/platform/windows_input.rs`
- Modify: `projects/vrv-desk/rust/src/bin/vrv_host.rs`
- Test: `projects/vrv-desk/rust/tests/input_shortcut_test.rs`

**Interfaces:**
- Consumes: `windows::Win32::UI::Input::KeyboardAndMouse::*`, `windows::Win32::System::DataExchange::*`
- Produces: `inject_unicode_text(text: &str)`, `inject_shortcut(name: &str)`, `get_clipboard_text() -> Option<String>`, `set_clipboard_text(text: &str) -> bool`

- [ ] **Step 1: Write the failing Rust integration test for shortcut and clipboard helpers**

Create `projects/vrv-desk/rust/tests/input_shortcut_test.rs` testing `set_clipboard_text` and `get_clipboard_text`.

- [ ] **Step 2: Run test to verify it fails**

Run `cargo test --test input_shortcut_test` and confirm missing functions.

- [ ] **Step 3: Implement Unicode text typing and shortcuts in `windows_input.rs`**

Add `inject_unicode_text` using `KEYEVENTF_UNICODE` SendInput, and `inject_shortcut` for `"win"`, `"esc"`, `"enter"`, `"backspace"`, `"task_manager"`, `"alt_tab"`, `"show_desktop"`.

- [ ] **Step 4: Implement Clipboard getter/setter and monitor in `vrv_host.rs`**

Add Win32 clipboard reading and writing, update `ClientInput` to parse `type_text`, `shortcut`, and `clipboard_text`, and broadcast Windows clipboard changes to client via `{"type": "clipboard_sync", "text": "..."}`.

- [ ] **Step 5: Run tests and verify compile**

Run `cargo test --test input_shortcut_test` and `cargo check --bin vrv_host`.

- [ ] **Step 6: Commit**

Commit changes with message `feat(host): add unicode typing, shortcuts, and clipboard sync`.

---

### Task 2: Flutter Virtual Keyboard & Shortcut Floating Toolbar

**Files:**
- Modify: `projects/vrv-desk/lib/src/views/mirror_view.dart`
- Create: `projects/vrv-desk/lib/src/widgets/shortcut_bar.dart`
- Test: `projects/vrv-desk/test/shortcut_bar_test.dart`

**Interfaces:**
- Consumes: `MirrorView` WebSocket sender
- Produces: `ShortcutBar` widget, soft-keyboard text input listener

- [ ] **Step 1: Write widget test for `ShortcutBar`**

Create `test/shortcut_bar_test.dart` asserting that tapping shortcut buttons (Win, Esc, Enter, Del, TaskMgr) calls `onShortcut(name)`.

- [ ] **Step 2: Run widget test to verify it fails**

Run `flutter test test/shortcut_bar_test.dart`.

- [ ] **Step 3: Implement `ShortcutBar` widget**

Build a sleek, horizontal glassmorphic bar with icon/text buttons for `Win`, `Esc`, `Enter`, `Del`, `TaskMgr`, `Alt+Tab`, and `Desktop`.

- [ ] **Step 4: Integrate `ShortcutBar` and Soft Keyboard in `MirrorView`**

Add a floating bottom dock in `MirrorView`:
- Keyboard button (`Icons.keyboard`): activates invisible `TextField` with `FocusNode` to open phone soft keyboard; sends characters as `type_text`.
- Shortcut button (`Icons.grid_view`): toggles visibility of `ShortcutBar`.
- Quick keys send `{ "type": "shortcut", "name": ... }`.

- [ ] **Step 5: Run tests and verify**

Run `flutter test` and `flutter analyze`.

- [ ] **Step 6: Commit**

Commit changes with message `feat(ui): add virtual keyboard bridge and floating shortcut bar in MirrorView`.

---

### Task 3: Bidirectional Clipboard Sync in Flutter & E2E Verification

**Files:**
- Modify: `projects/vrv-desk/lib/src/views/mirror_view.dart`
- Create: `projects/vrv-desk/test_e2e_clipboard.py`

**Interfaces:**
- Consumes: Flutter `Clipboard` API (`services.dart`), WebSocket connection
- Produces: Incoming clipboard sync listener and "Paste to PC" quick action

- [ ] **Step 1: Implement Clipboard handler in `MirrorView`**

Listen for incoming `{ "type": "clipboard_sync", "text": text }` from host:
- Call `Clipboard.setData(ClipboardData(text: text))`.
- Show bottom snackbar: `"📋 Copied from PC: <preview>"`.
Add a "Paste to PC" button in the floating dock:
- Reads `Clipboard.getData(Clipboard.kTextPlain)` and sends `{ "type": "clipboard_text", "text": text }` to PC.

- [ ] **Step 2: Rebuild Windows Host & Android Split APK**

Compile `vrv_host.exe` and rebuild `app-x86_64-release.apk` & `app-arm64-v8a-release.apk`.

- [ ] **Step 3: Write and run Python E2E verification test**

Verify bidirectional clipboard, text typing, and shortcuts against running `vrv_host.exe`.

- [ ] **Step 4: Test in Android Emulator against live PC**

Install APK on emulator, connect to Host, test typing text, pressing WinKey, and syncing clipboard.

- [ ] **Step 5: Commit and release v0.3.0**

Commit all changes, tag `v0.3.0`, and update GitHub release assets.
