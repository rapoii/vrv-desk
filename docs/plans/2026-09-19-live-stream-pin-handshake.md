# Implementation Plan: Dynamic 6-Digit PIN Handshake for Live Streaming (Phase 4)

**Goal:** Secure the live session streaming port 53211 with a dynamic 6-digit PIN authentication gatekeeper. Prevent unauthorized screen viewing and remote control until the client enters the matching PIN shown on the host PC.

---

## Architecture & Protocol

### 1. Host PIN Generation
- On startup, `vrv_host.exe` generates a random 6-digit numeric PIN (e.g., `849201`) using `rand::thread_rng().gen_range(100000..=999999)`.
- The PIN is prominently printed in the console banner and can be refreshed or verified.

### 2. WebSocket Handshake Sequence
1. **Client connects** to `ws://<host_ip>:53211`.
2. **Host responds** immediately with:
   ```json
   {
     "type": "auth_required",
     "host_name": "Host-PC",
     "version": "0.4.0"
   }
   ```
   *Note: Video capture loop (`frame_task`) and input injection (`input_task`) are NOT started yet.*
3. **Client sends** verification message:
   ```json
   {
     "type": "auth_verify",
     "pin": "849201"
   }
   ```
4. **Validation**:
   - **Match**: Host responds with:
     ```json
     {
       "type": "auth_ok",
       "session_token": "<random_hex_token>"
     }
     ```
     Host immediately starts `frame_task`, `clipboard_task`, and `input_task`.
   - **Mismatch**: Host increments failed attempts count. If attempts < 3:
     ```json
     {
       "type": "auth_failed",
       "reason": "Invalid PIN",
       "remaining_attempts": 2
     }
     ```
     If attempts >= 3, host sends `auth_failed` and terminates the WebSocket connection.

---

## Task Breakdown

### Task 1: Rust Host PIN Generation & Auth Handshake Gate
**Files:**
- Modify: `projects/vrv-desk/rust/src/bin/vrv_host.rs`
- Create: `projects/vrv-desk/rust/tests/auth_handshake_test.rs`

**Requirements:**
- Generate dynamic 6-digit PIN on host launch.
- Intercept WebSocket connection before frame capture starts.
- Enforce `auth_required` -> `auth_verify` -> `auth_ok` / `auth_failed` flow.
- Unit/integration test in `auth_handshake_test.rs` testing valid PIN, incorrect PIN, and lockout after 3 failed attempts.

### Task 2: Flutter Client Auth Handshake Protocol & PIN Prompt UI
**Files:**
- Modify: `projects/vrv-desk/lib/src/views/mirror_view.dart`
- Modify: `projects/vrv-desk/lib/src/views/home_view.dart`
- Create: `projects/vrv-desk/test/mirror_auth_test.dart`

**Requirements:**
- `MirrorView` accepts optional `initialPin`.
- Handle `auth_required` by displaying the PIN input dialog if not yet provided or showing an authentication state indicator.
- Automatically send `{"type": "auth_verify", "pin": pin}`.
- On `auth_ok`, transition to live streaming UI (`🟢 LIVE`).
- On `auth_failed`, display error message and allow re-entering PIN.
- Widget tests verifying authentication states and PIN submission.

### Task 3: Live End-to-End Verification on Android Emulator & Release v0.4.0
**Requirements:**
- Run E2E script `test_e2e_auth.py` verifying rejection of bad PIN and acceptance of valid PIN.
- Build split APK (`arm64-v8a`, `x86_64`).
- Test on Android Emulator: enter wrong PIN (verify error), enter correct PIN (verify live stream unlocks).
- Publish GitHub Release `v0.4.0`.
