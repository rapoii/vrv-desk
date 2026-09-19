use mirror_core::discovery::{
    LanBeacon, LanDiscoveryBroadcaster, DEFAULT_BROADCAST_INTERVAL_MS,
    DISCOVERY_MULTICAST_ADDR, DISCOVERY_PORT,
};
use std::net::UdpSocket;
use std::time::Duration;

#[test]
fn test_lan_beacon_serialization_roundtrip() {
    let beacon = LanBeacon::new(
        "123456".to_string(),
        "Test-Host".to_string(),
        53211,
    );

    assert_eq!(beacon.device_id, "123456");
    assert_eq!(beacon.device_name, "Test-Host");
    assert_eq!(beacon.os_type, "windows");
    assert_eq!(beacon.port, 53211);
    assert_eq!(beacon.protocol_version, 1);

    let bytes = beacon.encode();
    assert!(!bytes.is_empty());

    let decoded = LanBeacon::decode(&bytes).expect("Failed to decode LanBeacon");
    assert_eq!(decoded, beacon);
}

#[test]
fn test_lan_beacon_invalid_json() {
    let bad_data = b"{\"invalid\": \"json_data\"}";
    let result = LanBeacon::decode(bad_data);
    assert!(result.is_err(), "Expected error on missing required fields");

    let corrupted_data = b"not-json-at-all";
    let result2 = LanBeacon::decode(corrupted_data);
    assert!(result2.is_err(), "Expected error on corrupted bytes");
}

#[test]
fn test_lan_discovery_broadcast_receive_local() {
    // Check constants
    assert_eq!(DISCOVERY_MULTICAST_ADDR, "239.255.42.99");
    assert_eq!(DISCOVERY_PORT, 53210);
    assert_eq!(DEFAULT_BROADCAST_INTERVAL_MS, 1500);

    let beacon = LanBeacon::new(
        "TEST99".to_string(),
        "Discovery-Tester".to_string(),
        53211,
    );

    // Bind a listener socket on localhost or wildcard port to test socket creation & shutdown
    let listener = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind test UDP listener");
    listener
        .set_read_timeout(Some(Duration::from_millis(200)))
        .expect("Failed to set read timeout");

    // Start broadcaster with a fast interval (100ms) for testing
    let broadcaster = LanDiscoveryBroadcaster::start(beacon.clone(), 100);

    // Let it run for a short duration
    std::thread::sleep(Duration::from_millis(250));

    // Verify graceful stop and cleanup without hanging
    broadcaster.stop();
}

#[test]
fn test_lan_discovery_broadcaster_drop() {
    let beacon = LanBeacon::new(
        "DROP01".to_string(),
        "Drop-Tester".to_string(),
        53211,
    );

    let broadcaster = LanDiscoveryBroadcaster::start(beacon, 100);
    std::thread::sleep(Duration::from_millis(150));
    // Explicit drop should join thread cleanly
    drop(broadcaster);
}
