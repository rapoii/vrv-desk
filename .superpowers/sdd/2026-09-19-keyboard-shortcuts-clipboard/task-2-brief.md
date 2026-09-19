# Task 2 Brief: Flutter Virtual Keyboard & Shortcut Floating Toolbar

## Context
In `projects/vrv-desk/lib/src/views/mirror_view.dart`, the user currently has an interactive view of the PC screen and can control the mouse.
This task adds:
1. `projects/vrv-desk/lib/src/widgets/shortcut_bar.dart`:
   - A modern horizontal bar with glassmorphic styling (dark background `Colors.black87` with subtle rounded border).
   - Action buttons for:
     - `Win` (Start menu)
     - `Esc`
     - `Enter`
     - `Del` (Backspace)
     - `Tab`
     - `TaskMgr` (Task Manager)
     - `Alt+Tab`
     - `Desktop`
   - Callback: `ValueChanged<String> onShortcutPressed`
2. Update `MirrorView` in `lib/src/views/mirror_view.dart`:
   - A floating dock at the bottom of the screen with:
     - Keyboard button (`Icons.keyboard`): activates a hidden `TextField` with `FocusNode` to open Android's soft keyboard.
     - Keystrokes/characters entered in the `TextField` are sent to the host via `{"type": "type_text", "text": char}`.
     - Shortcuts toggle button (`Icons.grid_view`): toggles visibility of the `ShortcutBar`.
     - Direct shortcut buttons send `{"type": "shortcut", "name": name}` over the WebSocket.
3. Unit/Widget tests:
   - `test/shortcut_bar_test.dart` verifying all buttons render and trigger `onShortcutPressed`.

## Requirements
- Files to touch:
  - Create `lib/src/widgets/shortcut_bar.dart`
  - Modify `lib/src/views/mirror_view.dart`
  - Create `test/shortcut_bar_test.dart`
- Run `flutter test test/shortcut_bar_test.dart` and `flutter analyze` to verify.
- Report file: `.superpowers/sdd/2026-09-19-keyboard-shortcuts-clipboard/task-2-report.md`.
