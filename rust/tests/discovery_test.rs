use mirror_core::discovery::lan::{
    LanBeacon, LanDiscoveryBroadcaster, LanDiscoveryListener, DISCOVERY_MULTICAST_ADDR,
    DISCOVERY_PORT,
};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[test]
fn test_lan_beacon_roundtrip() {
    let beacon = LanBeacon {
        device_id: "849201".to_string(),
        device_name: "PC-Rafi".to_string(),
        os_type: "windows".to_string(),
        port: DISCOVERY_PORT,
        protocol_version: 1,
    };

    assert_eq!(DISCOVERY_MULTICAST_ADDR, Ipv4Addr::new(239, 255, 42, 99));
    let encoded = beacon.encode();
    let decoded = LanBeacon::decode(&encoded).expect("Valid beacon");
    assert_eq!(beacon, decoded);
}

#[test]
fn test_lan_discovery_helpers() {
    let beacon = LanBeacon::new(
        "849201".to_string(),
        "PC-Rafi".to_string(),
        "windows".to_string(),
        DISCOVERY_PORT,
    );
    let broadcaster = LanDiscoveryBroadcaster::new(beacon.clone());
    assert_eq!(broadcaster.beacon(), &beacon);

    let src_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)), DISCOVERY_PORT);
    let encoded = beacon.encode();
    let parsed = LanDiscoveryListener::parse_packet(&encoded, src_addr);
    assert_eq!(parsed, Some((beacon, src_addr)));

    let invalid = LanDiscoveryListener::parse_packet(b"not-json", src_addr);
    assert_eq!(invalid, None);
}
