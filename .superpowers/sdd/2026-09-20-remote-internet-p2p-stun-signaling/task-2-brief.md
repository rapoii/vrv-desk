# Task 2 Brief: Rust Signaling Broker & Host Remote Registration (Phase 5)

## Context & Objectives
In `projects/vrv-desk/rust`:
We are enabling remote access across different networks (e.g. home Wi-Fi and 4G mobile data) using a lightweight, high-performance signaling broker (`vrv_signal`) and 6-digit Device ID registration in `vrv_host`.

## Requirements
1. **Signaling Broker Binary `rust/src/bin/vrv_signal.rs`**:
   - High-performance async WebSocket server (default port `53212`, configurable via `--port` or `VRV_SIGNAL_PORT`).
   - Maintains registered hosts in an in-memory map:
     `registered_hosts: Arc<RwLock<HashMap<String, HostSession>>>`
   - Messages:
     - Host -> Signal:
       `{"type": "register_host", "device_id": "849201", "name": "Host-PC", "stun_endpoint": "114.10.41.71:31772"}`
       Signal -> Host:
       `{"type": "register_ok", "device_id": "849201"}`
     - Client -> Signal:
       `{"type": "connect_request", "target_id": "849201"}`
       If not found:
       Signal -> Client: `{"type": "connect_error", "reason": "Host 849201 not found or offline"}`
       If found:
       Signal bridges the client WebSocket connection with the host's remote streaming session!
2. **Host Remote Registration in `rust/src/bin/vrv_host.rs`**:
   - Add CLI flags:
     - `--device-id <ID>` (default: 6-digit hash or random 6 digits e.g. `849201`).
     - `--signal <URL>` (e.g. `ws://127.0.0.1:53212`).
   - On startup:
     - Query `StunClient::query_stun(DEFAULT_STUN_SERVER).await` (gracefully fallback if offline).
     - Connect to signaling broker (if `--signal` provided or default flag enabled) and register `device_id`.
     - Update startup banner:
       ```
       =================================================
       🌐 VrV Desk Remote Host Ready!
       📱 Device ID:   849 201
       🔐 Session PIN: 482 910
       📡 STUN Public: 114.10.41.71:31772
       📶 Local LAN:   ws://0.0.0.0:53211
       =================================================
       ```
3. **Integration Test `rust/tests/signaling_test.rs`**:
   - Test starting signaling broker, host registering device ID, client querying and connecting to host ID.
4. **Verification**:
   - `cargo test --test signaling_test` passes cleanly.
