# Task 1 Brief: Rust Host PIN Generation & Auth Handshake Gate

## Context & Objectives
In `projects/vrv-desk/rust`:
Currently `src/bin/vrv_host.rs` accepts any incoming WebSocket client and immediately starts the screen capturer and input injection without any security verification.
In this task, implement a secure dynamic 6-digit PIN handshake gate before the streaming and input tasks are started.

## Detailed Requirements
1. **Dynamic PIN Generation & Configuration:**
   - In `vrv_host.rs`, check for environment variable `VRV_PIN` or CLI argument `--pin <PIN>`. If not provided, generate a secure random 6-digit numeric PIN:
     `format!("{:06}", rand::thread_rng().gen_range(100000..=999999))`
   - Print the PIN clearly on host startup:
     ```
     =================================================
     🔐 Host Ready!
     📱 Session PIN: 849 201
     🌐 Listening on ws://0.0.0.0:53211
     =================================================
     ```
   - Store the active PIN in `Arc<String>` (or pass by reference) so `handle_connection` can access it.

2. **WebSocket Authentication Protocol:**
   - When a client connects and WebSocket handshake completes (`accept_async`), DO NOT start `ScreenCapturer` or any streaming yet.
   - Send initial text message:
     ```json
     {
       "type": "auth_required",
       "host_name": "Host-PC",
       "version": "0.4.0"
     }
     ```
   - Enter an authentication verification loop with a 20-second timeout per message:
     - Expect a text message with JSON structure:
       ```json
       {
         "type": "auth_verify",
         "pin": "849201"
       }
       ```
     - **If PIN matches:**
       - Generate a session token (e.g. `hex::encode(rand::random::<[u8; 16]>())`).
       - Send response:
         ```json
         {
           "type": "auth_ok",
           "session_token": "<token>"
         }
         ```
       - Print: `✅ Client <addr> authenticated successfully!`
       - Break the auth loop and proceed to `ScreenCapturer::new()` and launch `frame_task`, `clipboard_task`, and `input_task` as before!
     - **If PIN does not match:**
       - Increment failed attempts counter (initial remaining = 3).
       - If remaining > 0:
         - Send response:
           ```json
           {
             "type": "auth_failed",
             "reason": "Invalid PIN",
             "remaining_attempts": remaining
           }
           ```
         - Print: `⚠️ Client <addr> entered invalid PIN. Remaining attempts: <remaining>`
       - If remaining == 0:
         - Send response:
           ```json
           {
             "type": "auth_failed",
             "reason": "Too many failed attempts. Disconnecting.",
             "remaining_attempts": 0
           }
           ```
         - Disconnect the WebSocket client and return an error.

3. **Integration Test (`tests/auth_handshake_test.rs`):**
   - Write integration test using `tokio-tungstenite` connecting to an in-process or mock listener testing:
     - Successful auth with correct PIN receives `auth_required`, sends `auth_verify`, receives `auth_ok`.
     - Invalid PIN receives `auth_failed` with remaining attempts decremented.
     - 3 failed attempts results in disconnection / termination.
   - Verify with `cargo test --test auth_handshake_test`.

4. **Safety & Toolchain Rules:**
   - Always export toolchain path in bash before running cargo:
     `export PATH="/c/Users/Rafi/Software/mingw64/bin:$HOME/.cargo/bin:$PATH"`
   - Target dir: compile cleanly with 0 warnings or errors.
   - Recompile `cargo build --bin vrv_host` and copy to `dist/windows/vrv_host.exe`.
   - Write execution report to `projects/vrv-desk/.superpowers/sdd/2026-09-19-live-stream-pin-handshake/task-1-report.md`.
