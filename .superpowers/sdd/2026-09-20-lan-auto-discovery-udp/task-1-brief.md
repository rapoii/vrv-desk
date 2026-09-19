# Task 1 Brief: Rust LAN Discovery Broadcaster & LanBeacon Module

## Context & Objectives
You are implementing Task 1 of Phase 7 (Zero-Config LAN Discovery Engine via UDP) in `projects/vrv-desk/rust`.
This enables the Windows host PC running `vrv_host.exe` to broadcast its presence over local Wi-Fi / LAN so Android devices on the same network can discover it automatically without typing IP addresses.

## Requirements
1. **Create `rust/src/discovery/mod.rs` and `rust/src/discovery/lan.rs`**:
   - Define `LanBeacon`:
     ```rust
     use serde::{Deserialize, Serialize};

     pub const DISCOVERY_MULTICAST_ADDR: &str = "239.255.42.99";
     pub const DISCOVERY_PORT: u16 = 53210;
     pub const DEFAULT_BROADCAST_INTERVAL_MS: u64 = 1500;

     #[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
     pub struct LanBeacon {
         pub device_id: String,
         pub device_name: String,
         pub os_type: String,
         pub port: u16,
         pub protocol_version: u32,
     }

     impl LanBeacon {
         pub fn new(device_id: String, device_name: String, port: u16) -> Self {
             Self {
                 device_id,
                 device_name,
                 os_type: "windows".to_string(),
                 port,
                 protocol_version: 1,
             }
         }

         pub fn encode(&self) -> Vec<u8> {
             serde_json::to_vec(self).unwrap_or_default()
         }

         pub fn decode(bytes: &[u8]) -> Result<Self, serde_json::Error> {
             serde_json::from_slice(bytes)
         }
     }
     ```
   - Implement `LanDiscoveryBroadcaster`:
     - Spawns a background worker thread (`std::thread::spawn`).
     - Binds a UDP socket to `0.0.0.0:0` (ephemeral port).
     - Sets `socket.set_broadcast(true)`.
     - Sets multicast TTL (`socket.set_multicast_ttl_v4(4)` or similar).
     - In a loop, sends `beacon.encode()` to:
       1. Multicast address: `239.255.42.99:53210`
       2. Subnet broadcast address: `255.255.255.255:53210`
     - Sleeps for `Duration::from_millis(interval_ms)` (default 1500ms).
     - Provides a graceful shutdown signal via `Arc<AtomicBool>`.
     - Exposes:
       ```rust
       pub struct LanDiscoveryBroadcaster {
           stop_signal: Arc<AtomicBool>,
           handle: Option<std::thread::JoinHandle<()>>,
       }

       impl LanDiscoveryBroadcaster {
           pub fn start(beacon: LanBeacon, interval_ms: u64) -> Self;
           pub fn stop(mut self);
       }
       impl Drop for LanDiscoveryBroadcaster {
           fn drop(&mut self) { ... }
       }
       ```

2. **Register module in `rust/src/lib.rs`**:
   - Add `pub mod discovery;`

3. **Integrate into `rust/src/bin/vrv_host.rs`**:
   - In `main()` of `vrv_host.rs`:
     - Read machine name:
       ```rust
       let device_name = std::env::var("COMPUTERNAME")
           .or_else(|_| std::env::var("HOSTNAME"))
           .unwrap_or_else(|_| "Windows PC".to_string());
       ```
     - Construct `LanBeacon::new(device_id.clone(), device_name, port)` (port is 53211).
     - Start broadcaster: `let _broadcaster = LanDiscoveryBroadcaster::start(beacon, 1500);`.
     - Log: `println!("📡 LAN discovery broadcaster started on UDP port {} (multicast: {})", DISCOVERY_PORT, DISCOVERY_MULTICAST_ADDR);`.

4. **Write unit and integration tests in `rust/tests/discovery_test.rs`**:
   - Test 1: `test_lan_beacon_serialization_roundtrip` (encode and decode).
   - Test 2: `test_lan_beacon_invalid_json` (graceful error handling).
   - Test 3: `test_lan_discovery_broadcast_receive_local`:
     - Bind a test UDP listener to `127.0.0.1:0`.
     - Verify encoding, broadcast start, and stopping without hang or panic.

5. **Toolchain & Verification**:
   - Cargo toolchain: `/c/Users/Rafi/.cargo/bin/cargo` (or `cargo`).
   - Run `cargo test --test discovery_test`.
   - Run `cargo check --bin vrv_host`.
   - Run all tests `cargo test`.
   - Compile `cargo build --release --bin vrv_host` and copy binary to `dist/windows/vrv_host.exe`.

## Output Contract
Write full report to:
`projects/vrv-desk/.superpowers/sdd/2026-09-20-lan-auto-discovery-udp/task-1-report.md`
Report must contain:
- Files created and modified.
- Test commands run and exact outputs.
- Confirmation that `cargo test` passed 100%.
