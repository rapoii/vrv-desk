# Task 1 Brief: Rust Host Keyboard Typing, Shortcuts & Win32 Clipboard Support

## Context
In `projects/vrv-desk`, the Rust core and `vrv_host.rs` provide the screen streaming and mouse input server on port 53211. This task extends it with:
1. `inject_unicode_text(text: &str)` using Win32 `SendInput` with `KEYEVENTF_UNICODE` so arbitrary text/words typed on Android are typed directly on Windows.
2. `inject_shortcut(name: &str)` supporting:
   - `"win"`: Tap VK_LWIN (0x5B)
   - `"esc"`: Tap VK_ESCAPE (0x1B)
   - `"enter"`: Tap VK_RETURN (0x0D)
   - `"backspace"`: Tap VK_BACK (0x08)
   - `"tab"`: Tap VK_TAB (0x09)
   - `"task_manager"`: Ctrl + Shift + Esc (VK_CONTROL, VK_SHIFT, VK_ESCAPE)
   - `"alt_tab"`: Alt + Tab (VK_MENU, VK_TAB)
   - `"show_desktop"`: Win + D (VK_LWIN, 0x44)
3. Win32 Clipboard helper functions:
   - `get_clipboard_text() -> Option<String>`
   - `set_clipboard_text(text: &str) -> bool`
   Using `windows::Win32::System::DataExchange` (`OpenClipboard`, `CloseClipboard`, `GetClipboardData`, `SetClipboardData`, `EmptyClipboard`, `CF_UNICODETEXT`) and `GlobalAlloc`/`GlobalLock`.
4. Update `ClientInput` in `src/bin/vrv_host.rs`:
   - `type_text`: calls `inject_unicode_text(&text)`
   - `shortcut`: calls `inject_shortcut(&name)`
   - `clipboard_text`: calls `set_clipboard_text(&text)`
5. Clipboard monitor loop in `vrv_host.rs`:
   - Checks `GetClipboardSequenceNumber()` every ~500ms.
   - If changed, sends `{"type": "clipboard_sync", "text": "..."}` to connected client.

## Requirements
- Rust toolchain: MinGW gcc is at `/c/Users/Rafi/Software/mingw64/bin`, cargo at `/c/Users/Rafi/.cargo/bin`. Use `export PATH="/c/Users/Rafi/Software/mingw64/bin:$HOME/.cargo/bin:$PATH"`.
- Working directory: `D:/Software/Hermes Workspace/projects/vrv-desk/rust`.
- Files to touch:
  - `rust/Cargo.toml` (ensure features `Win32_System_DataExchange` and `Win32_System_Memory` are present)
  - `rust/src/platform/windows_input.rs`
  - `rust/src/bin/vrv_host.rs`
  - `rust/tests/input_shortcut_test.rs`
- Run `cargo test --test input_shortcut_test` and `cargo check --bin vrv_host` to verify.
- Report file: `.superpowers/sdd/2026-09-19-keyboard-shortcuts-clipboard/task-1-report.md`.
