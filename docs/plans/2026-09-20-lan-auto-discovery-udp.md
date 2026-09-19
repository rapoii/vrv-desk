# Implementation Plan: Zero-Config LAN Discovery Engine via UDP (Phase 7 / Task 5)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement zero-config LAN device discovery so that when a Windows PC running `vrv_host.exe` and an Android device running the VrV Desk app are connected to the same Wi-Fi network, the PC automatically appears in the Android `HomeView` device list without manual IP or Device ID entry.

**Architecture:**
- **Host (Rust)**: Spawns a background UDP broadcaster sending a serialized JSON `LanBeacon` every 1.5 seconds to multicast address `239.255.42.99:53210` and subnet broadcast `255.255.255.255:53210`.
- **Client (Flutter / Dart `dart:io`)**: Runs `LanDiscoveryService` binding `RawDatagramSocket` to UDP port `53210`, joins multicast `239.255.42.99`, receives beacons, resolves sender IP, maintains active device cache, and auto-prunes stale devices (>6s).
- **Presentation (`HomeView`)**: Subscribes to `LanDiscoveryService.devicesStream`, displaying discovered PC cards with OS icon, name, IP, and a 1-tap "Connect" button opening the PIN dialog.

**Tech Stack:**
- Rust (`std::net::UdpSocket`, `serde`, `serde_json`)
- Flutter / Dart (`dart:io` `RawDatagramSocket`, `StreamController`)
- Material 3 Flutter UI (`HomeView`, `DiscoveredDevice`)

**Spec:** `docs/IMPLEMENTATION_PLAN.md` (Task 5) & `docs/ARCHITECTURE.md` (Section 4.1)

## Global Constraints
- Multicast address: `239.255.42.99`
- Discovery UDP Port: `53210`
- Service Port: `53211` (WebSocket stream)
- Protocol Version: `1`
- Broadcast Interval: 1.5 seconds
- Stale Device Timeout: 6.0 seconds
- Zero third-party Dart package dependencies (use standard `dart:io` `RawDatagramSocket`)

---

## Tasks Breakdown

### Task 1: Rust LAN Discovery Broadcaster & LanBeacon Module
**Files:**
- Create: `projects/vrv-desk/rust/src/discovery/mod.rs`
- Create: `projects/vrv-desk/rust/src/discovery/lan.rs`
- Modify: `projects/vrv-desk/rust/src/lib.rs`
- Modify: `projects/vrv-desk/rust/src/bin/vrv_host.rs`
- Create: `projects/vrv-desk/rust/tests/discovery_test.rs`

**Interfaces:**
- Consumes: `serde`, `serde_json`, `std::net::UdpSocket`
- Produces: `LanBeacon`, `LanDiscoveryBroadcaster::start(beacon, interval_ms)`

- [ ] **Step 1: Write failing unit test for `LanBeacon` serialization and UDP broadcast loop**
  In `rust/tests/discovery_test.rs`:
  - Test encoding `LanBeacon` to JSON bytes and decoding back.
  - Test validation of fields: `device_id`, `device_name`, `os_type`, `port`, `protocol_version`.
  - Test simulated UDP send/receive on `127.0.0.1`.

- [ ] **Step 2: Run test to verify it fails**
  Run `cargo test --test discovery_test` to confirm compilation/test failure.

- [ ] **Step 3: Implement `LanBeacon` and `LanDiscoveryBroadcaster`**
  - Implement struct `LanBeacon` in `rust/src/discovery/lan.rs`.
  - Implement `start_broadcaster(beacon: LanBeacon, interval: Duration) -> LanDiscoveryBroadcaster` which spawns a dedicated daemon thread broadcasting to `239.255.42.99:53210` and `255.255.255.255:53210`.
  - Export `pub mod discovery;` in `rust/src/lib.rs`.

- [ ] **Step 4: Integrate broadcaster into `rust/src/bin/vrv_host.rs`**
  - On host startup (after generating Device ID and PIN), construct `LanBeacon` using the machine's hostname (via `std::env::var("COMPUTERNAME")` or `whoami` fallback) and start the broadcast thread.

- [ ] **Step 5: Run tests and verify compile**
  Run `cargo test` and `cargo check --bin vrv_host`.

- [ ] **Step 6: Commit**
  ```bash
  git add rust/src/discovery rust/src/lib.rs rust/src/bin/vrv_host.rs rust/tests/discovery_test.rs
  git commit -m "feat: implement Rust LAN UDP discovery beacon and host broadcaster"
  ```

---

### Task 2: Flutter LAN Discovery Service & HomeView Live Device Integration
**Files:**
- Create: `projects/vrv-desk/lib/src/services/lan_discovery_service.dart`
- Modify: `projects/vrv-desk/lib/src/views/home_view.dart`
- Create: `projects/vrv-desk/test/lan_discovery_test.dart`

**Interfaces:**
- Consumes: `dart:io` `RawDatagramSocket`, `DiscoveredDevice`
- Produces: `LanDiscoveryService`, `HomeView` live device list auto-population

- [ ] **Step 1: Write unit test for `LanDiscoveryService` parsing and device lifecycle**
  In `test/lan_discovery_test.dart`:
  - Test parsing beacon datagram into `DiscoveredDevice`.
  - Test device deduplication and timestamp updating.
  - Test pruning of devices older than 6 seconds.

- [ ] **Step 2: Run test to verify it fails**
  Run `flutter test test/lan_discovery_test.dart`.

- [ ] **Step 3: Implement `LanDiscoveryService`**
  - In `lib/src/services/lan_discovery_service.dart`:
    - Bind `RawDatagramSocket` to port `53210`.
    - Join multicast group `239.255.42.99`.
    - Listen for `RawSocketEvent.read`, read `socket.receive()`.
    - Parse JSON datagram, update internal map `Map<String, DiscoveredDevice>`.
    - Periodic timer (every 2s) to prune devices older than 6s.
    - Expose `Stream<List<DiscoveredDevice>> get devicesStream`.

- [ ] **Step 4: Integrate `LanDiscoveryService` into `HomeView`**
  - In `lib/src/views/home_view.dart`:
    - Instantiate and start `LanDiscoveryService` on `initState()`.
    - Listen to `devicesStream` and update `_devices` via `setState()`.
    - Discovered devices appear as interactive cards in `LAN Devices` section.
    - Tapping a device opens `_showPinDialog(device)` and connects directly to `MirrorView(hostIp: device.ipAddress, port: device.port, initialPin: pin)`.
    - Stop service on `dispose()`.

- [ ] **Step 5: Run tests and verify**
  Run `flutter test` and `flutter analyze`.

- [ ] **Step 6: Commit**
  ```bash
  git add lib/src/services/lan_discovery_service.dart lib/src/views/home_view.dart test/lan_discovery_test.dart
  git commit -m "feat: implement Flutter LAN discovery service and HomeView live auto-detection"
  ```

---

### Task 3: Integration, E2E Verification & Release v0.7.0
**Files:**
- Create: `projects/vrv-desk/test_e2e_lan_discovery.py`
- Modify: `projects/vrv-desk/docs/IMPLEMENTATION_PLAN.md` (check Task 5)

- [ ] **Step 1: Write Python E2E verification test for UDP discovery**
  In `test_e2e_lan_discovery.py`:
  - Start UDP listener on port `53210` joining multicast `239.255.42.99`.
  - Launch `vrv_host.exe`.
  - Verify that a valid `LanBeacon` JSON is received within 3 seconds containing correct `device_id`, `os_type: "windows"`, and port `53211`.

- [ ] **Step 2: Rebuild Windows Host & Android Split APK**
  - Build `vrv_host.exe` and copy to `dist/windows/`.
  - Build `flutter build apk --release --split-per-abi`.
  - Install `vrv-desk-v0.7.0-x86_64.apk` on `emulator-5554`.

- [ ] **Step 3: Live Verification on Emulator**
  - Verify auto-discovery UI or simulated beacon in `HomeView`.
  - Take screenshot showing discovered PC in LAN Devices card.

- [ ] **Step 4: Commit, Tag and Publish Release v0.7.0**
  - Tag `v0.7.0` and publish GitHub release.
