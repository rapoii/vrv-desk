### What's New in v0.7.0:
- **Zero-Config LAN Auto-Discovery via UDP (Task 5 Master Plan)**:
  - **Windows Host (`rust/src/discovery/lan.rs`)**: Automatic background UDP broadcaster that periodically sends serialized `LanBeacon` packets every 1.5 seconds to multicast address `239.255.42.99:53210` and subnet broadcast `255.255.255.255:53210`.
  - **Android Client (`lib/src/services/lan_discovery_service.dart`)**: Background UDP discovery service listening on port 53210, parsing beacons, resolving sender host IP, and maintaining a real-time reactive device cache.
  - **Interactive HomeView Integration (`home_view.dart`)**: Detected PCs on the same local Wi-Fi network automatically appear as interactive device cards with operating system badges, device names, and IP addresses.
  - **1-Tap Quick Connect**: Tapping any detected PC card immediately initiates the pairing flow (opening the PIN dialog) without needing to manually discover or type local IP addresses.
  - **Stale Device Eviction**: Automatically removes offline PCs from the UI if no heartbeat beacon is detected within 6 seconds.
