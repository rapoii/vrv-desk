# Task 3 Brief: Flutter Client Remote Device ID Connect (Phase 5)

## Context & Objectives
In `projects/vrv-desk`:
We want users to connect to their remote PC simply by entering the PC's 6-digit Device ID (e.g. `849 201`) into the `HomeView` Quick Connect input, without needing to know IP addresses or port forwarding.

## Requirements
1. **Detection in `lib/src/views/home_view.dart`**:
   - In Quick Connect / Direct Connect:
     - Check if input is a 6-digit numeric Device ID (e.g. `849201` or `849 201` after stripping spaces):
       - If 6-digit Device ID: Route connection through Signaling Broker (e.g. `ws://10.0.2.2:53212` on emulator or configured signaling URL).
       - If IP address / hostname: Route directly to `ws://<ip>:53211`.
2. **Support in `lib/src/views/mirror_view.dart`**:
   - Allow connecting via signaling session URL / WebSocket channel.
   - Transparently run the exact same `auth_required` -> `auth_verify` -> `auth_ok` flow.
   - Receive live video frames and send touch/keyboard/clipboard events.
3. **Tests in `test/remote_connect_test.dart`**:
   - Verify parsing and routing of 6-digit Device ID vs direct IP.
   - Verify widget flow for Quick Connect with Device ID.
4. **Verification**:
   - `flutter test` and `flutter analyze` 0 errors.
