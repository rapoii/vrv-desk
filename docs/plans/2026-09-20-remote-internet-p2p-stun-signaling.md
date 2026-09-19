# Implementation Plan: Remote Internet P2P via STUN & Signaling (Phase 5)

**Goal:** Enable seamless remote screen mirroring and control across different networks (e.g., PC on home Wi-Fi and Android phone on 4G cellular) using STUN NAT traversal (`stun.l.google.com:19302`) and a lightweight 6-digit Device ID signaling broker.

## Architecture & Flow
1. **Host PC (`vrv_host`):**
   - Resolves public reflexive endpoint via STUN binding request to `stun.l.google.com:19302`.
   - Derives or registers a 6-digit Device ID (e.g. `849 201`).
   - Registers with the signaling broker (`vrv_signal`).
   - Displays both Local LAN IP and Remote Device ID on console banner.
2. **Signaling Broker (`vrv_signal`):**
   - High-performance WebSocket rendezvous server.
   - Matches target 6-digit Device ID and proxies the initial handshake between peers.
3. **Android Client (`MirrorView` / `HomeView`):**
   - In `HomeView`, users can type the 6-digit Device ID or scan QR code.
   - Connects to the host through the signaling broker.
   - Enforces the 6-digit dynamic PIN handshake (`auth_required` -> `auth_verify` -> `auth_ok`).
   - Streams live video frames, touch/mouse events, keyboard, and clipboard.

## Tasks Breakdown
- **Task 1: Rust RFC 5389 STUN Client** (`rust/src/stun.rs`, `rust/tests/stun_test.rs`)
- **Task 2: Rust Signaling Broker & Host Remote Registration** (`rust/src/bin/vrv_signal.rs`, `rust/src/bin/vrv_host.rs`, `rust/tests/signaling_test.rs`)
- **Task 3: Flutter Client Remote Device ID Connect** (`home_view.dart`, `mirror_view.dart`, `test/remote_connect_test.dart`)
- **Task 4: Live E2E Verification & Release v0.5.0**
