pub mod lan;

pub use lan::{
    LanBeacon, LanDiscoveryBroadcaster, DEFAULT_BROADCAST_INTERVAL_MS, DISCOVERY_MULTICAST_ADDR,
    DISCOVERY_PORT,
};
