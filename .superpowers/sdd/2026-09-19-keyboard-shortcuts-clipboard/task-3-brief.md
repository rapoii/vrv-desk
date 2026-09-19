# Task 3 Brief: Bidirectional Clipboard Sync in Flutter & E2E Verification

## Context
In this task:
1. `MirrorView` in `lib/src/views/mirror_view.dart`:
   - Adds incoming WebSocket handler for `{ "type": "clipboard_sync", "text": text }`:
     - Calls `Clipboard.setData(ClipboardData(text: text))`.
     - Shows an unobtrusive SnackBar: `"📋 Clipboard synced from PC"`.
   - Adds a "Paste to PC" button in the floating dock:
     - Reads `Clipboard.getData(Clipboard.kTextPlain)` and sends `{ "type": "clipboard_text", "text": text }` over WebSocket to PC.
2. End-to-End Verification:
   - Compile `vrv_host.exe`.
   - Rebuild Android split APK (`x86_64` for emulator, `arm64-v8a` for mobile).
   - Install APK on emulator.
   - Run Python E2E verification test `test_e2e_clipboard.py` testing:
     - Sending `type_text` -> verifies Win32 input
     - Sending `shortcut` ("win", "task_manager") -> verifies Win32 input
     - Sending `clipboard_text` -> verifies Windows clipboard updated
     - Verifying `clipboard_sync` broadcast received from host.
   - Live test via `adb` on emulator.
3. Release packaging:
   - Rebuild release artifacts, commit and push to Git.

## Requirements
- Files to touch:
  - `lib/src/views/mirror_view.dart`
  - `test_e2e_clipboard.py`
- Run `flutter test`, `flutter analyze`, and run `test_e2e_clipboard.py` against `vrv_host.exe`.
- Report file: `.superpowers/sdd/2026-09-19-keyboard-shortcuts-clipboard/task-3-report.md`.
