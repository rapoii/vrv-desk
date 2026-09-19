use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddr};

pub const DISCOVERY_MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(239, 255, 42, 99);
pub const DISCOVERY_PORT: u16 = 53210;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct LanBeacon {
    pub device_id: String,
    pub device_name: String,
    pub os_type: String,
    pub port: u16,
    pub protocol_version: u32,
}

impl LanBeacon {
    pub fn new(device_id: String, device_name: String, os_type: String, port: u16) -> Self {
        Self {
            device_id,
            device_name,
            os_type,
            port,
            protocol_version: 1,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    pub fn decode(data: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(data)
    }
}

/// Helper struct for broadcasting discovery beacons over UDP multicast.
pub struct LanDiscoveryBroadcaster {
    beacon: LanBeacon,
}

impl LanDiscoveryBroadcaster {
    pub fn new(beacon: LanBeacon) -> Self {
        Self { beacon }
    }

    pub fn beacon(&self) -> &LanBeacon {
        &self.beacon
    }
}

/// Helper struct for listening to discovery beacons over UDP multicast.
pub struct LanDiscoveryListener;

impl LanDiscoveryListener {
    pub fn parse_packet(data: &[u8], src_addr: SocketAddr) -> Option<(LanBeacon, SocketAddr)> {
        LanBeacon::decode(data).ok().map(|b| (b, src_addr))
    }
}
