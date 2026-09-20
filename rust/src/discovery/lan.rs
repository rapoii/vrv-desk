use serde::{Deserialize, Serialize};
use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

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
    #[serde(default)]
    pub host_ip: Option<String>,
}

impl LanBeacon {
    pub fn new(device_id: String, device_name: String, port: u16) -> Self {
        Self {
            device_id,
            device_name,
            os_type: "windows".to_string(),
            port,
            protocol_version: 1,
            host_ip: None,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

pub struct LanDiscoveryBroadcaster {
    stop_signal: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl LanDiscoveryBroadcaster {
    pub fn start(beacon: LanBeacon, interval_ms: u64) -> Self {
        let stop_signal = Arc::new(AtomicBool::new(false));
        let stop_signal_clone = stop_signal.clone();

        let handle = std::thread::spawn(move || {
            let socket = match UdpSocket::bind("0.0.0.0:0") {
                Ok(s) => s,
                Err(e) => {
                    eprintln!(
                        "[LanDiscoveryBroadcaster] Failed to bind UDP socket: {:?}",
                        e
                    );
                    return;
                }
            };

            if let Err(e) = socket.set_broadcast(true) {
                eprintln!("[LanDiscoveryBroadcaster] Warning: Failed to set broadcast on UDP socket: {:?}", e);
            }

            if let Err(e) = socket.set_multicast_ttl_v4(4) {
                eprintln!(
                    "[LanDiscoveryBroadcaster] Warning: Failed to set multicast TTL: {:?}",
                    e
                );
            }

            let multicast_target: Result<SocketAddr, _> =
                format!("{}:{}", DISCOVERY_MULTICAST_ADDR, DISCOVERY_PORT).parse();
            let broadcast_target: Result<SocketAddr, _> =
                format!("255.255.255.255:{}", DISCOVERY_PORT).parse();

            let payload = beacon.encode();

            while !stop_signal_clone.load(Ordering::Relaxed) {
                if let Ok(target) = multicast_target {
                    let _ = socket.send_to(&payload, target);
                }
                if let Ok(target) = broadcast_target {
                    let _ = socket.send_to(&payload, target);
                }

                // Sleep in small increments so shutdown is responsive
                let sleep_total = Duration::from_millis(interval_ms);
                let sleep_step = Duration::from_millis(50);
                let mut elapsed = Duration::from_millis(0);

                while elapsed < sleep_total {
                    if stop_signal_clone.load(Ordering::Relaxed) {
                        break;
                    }
                    let to_sleep = if sleep_total - elapsed < sleep_step {
                        sleep_total - elapsed
                    } else {
                        sleep_step
                    };
                    std::thread::sleep(to_sleep);
                    elapsed += to_sleep;
                }
            }
        });

        Self {
            stop_signal,
            handle: Some(handle),
        }
    }

    pub fn stop(mut self) {
        self.stop_signal.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for LanDiscoveryBroadcaster {
    fn drop(&mut self) {
        self.stop_signal.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
