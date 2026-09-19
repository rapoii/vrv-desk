# Task 2 Brief: Flutter Client Auth Handshake Protocol & PIN Prompt UI

## Context & Objectives
In `projects/vrv-desk`:
The Flutter client connects to the Windows Host via WebSocket in `MirrorView` (`lib/src/views/mirror_view.dart`).
Now that the host enforces a dynamic 6-digit PIN authentication handshake, `MirrorView` must handle:
1. Receiving `auth_required` from the host.
2. Sending `auth_verify` with the 6-digit PIN.
3. Handling `auth_ok` to begin stream rendering and controls.
4. Handling `auth_failed` to show remaining attempts or error, and letting user retry or cancel.

## Detailed Requirements
1. **`MirrorView` Modifications (`lib/src/views/mirror_view.dart`):**
   - Add parameter: `final String? initialPin;` to `MirrorView` constructor.
   - Add state: `bool _isAuthenticated = false;`, `String? _authError;`, `int _remainingAttempts = 3;`.
   - In WebSocket message listener:
     - Check for text messages before / during stream:
       - If message contains `"type": "auth_required"`:
         - If `widget.initialPin != null` and not yet sent:
           - Immediately send: `jsonEncode({"type": "auth_verify", "pin": widget.initialPin})`.
         - Otherwise:
           - Trigger the PIN entry dialog or overlay (`_showPinDialog()`).
       - If message contains `"type": "auth_ok"`:
         - Set `_isAuthenticated = true;`.
         - Dismiss any open PIN dialog.
         - Show brief SnackBar: "Connected & Authenticated with Host PC".
       - If message contains `"type": "auth_failed"`:
         - Parse `reason` and `remaining_attempts`.
         - Update state `_authError = reason` and `_remainingAttempts = remaining_attempts`.
         - If `remaining_attempts <= 0`:
           - Show fatal error SnackBar and close/pop `MirrorView` after short delay.
   - While `!_isAuthenticated`:
     - Render an authentication status / PIN prompt overlay in the center of the screen instead of black/loading screen.
     - If user enters PIN in the overlay/dialog, send: `jsonEncode({"type": "auth_verify", "pin": enteredPin})`.

2. **`HomeView` Enhancements (`lib/src/views/home_view.dart`):**
   - In `_showDirectIpDialog()`:
     - Add an optional PIN input field (`Host PIN (Optional, 6 digits)`).
     - Pass the PIN to `MirrorView(hostIp: ip, port: 53211, initialPin: pin.isNotEmpty ? pin : null)`.
   - In `_showPinDialog()`:
     - Pass the entered PIN to `MirrorView(hostIp: device.ipAddress, port: device.port, initialPin: pin)`.

3. **Widget & Unit Tests (`test/mirror_auth_test.dart`):**
   - Test that `MirrorView` handles `auth_required` message.
   - Test that `MirrorView` automatically sends `auth_verify` when `initialPin` is supplied.
   - Test that `MirrorView` displays the PIN dialog/prompt when `initialPin` is null.
   - Verify `flutter test` passes 100% cleanly.
   - Verify `flutter analyze` has 0 issues.

4. **Output Report:**
   - Write execution report to `projects/vrv-desk/.superpowers/sdd/2026-09-19-live-stream-pin-handshake/task-2-report.md`.
