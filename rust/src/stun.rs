use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time::timeout;

pub const DEFAULT_STUN_SERVER: &str = "stun.l.google.com:19302";
pub const MAGIC_COOKIE: u32 = 0x2112_A442;
pub const BINDING_REQUEST: u16 = 0x0001;
pub const BINDING_RESPONSE: u16 = 0x0101;
pub const ATTR_MAPPED_ADDRESS: u16 = 0x0001;
pub const ATTR_XOR_MAPPED_ADDRESS: u16 = 0x0020;

#[derive(Debug)]
pub enum StunError {
    Io(std::io::Error),
    Timeout,
    InvalidResponse(String),
    AddressFamilyNotSupported(u8),
    MappedAddressNotFound,
}

impl fmt::Display for StunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StunError::Io(e) => write!(f, "IO error: {}", e),
            StunError::Timeout => write!(f, "STUN request timed out"),
            StunError::InvalidResponse(msg) => write!(f, "Invalid STUN response: {}", msg),
            StunError::AddressFamilyNotSupported(fam) => {
                write!(f, "Unsupported address family: 0x{:02X}", fam)
            }
            StunError::MappedAddressNotFound => {
                write!(f, "Neither XOR-MAPPED-ADDRESS nor MAPPED-ADDRESS found")
            }
        }
    }
}

impl std::error::Error for StunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StunError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for StunError {
    fn from(err: std::io::Error) -> Self {
        StunError::Io(err)
    }
}

pub struct StunClient;

impl StunClient {
    /// Builds a 20-byte RFC 5389 Binding Request packet with the given 12-byte transaction ID.
    pub fn build_binding_request(transaction_id: &[u8; 12]) -> [u8; 20] {
        let mut msg = [0u8; 20];
        msg[0..2].copy_from_slice(&BINDING_REQUEST.to_be_bytes());
        msg[2..4].copy_from_slice(&0x0000u16.to_be_bytes());
        msg[4..8].copy_from_slice(&MAGIC_COOKIE.to_be_bytes());
        msg[8..20].copy_from_slice(transaction_id);
        msg
    }

    /// Parses a STUN response payload and extracts the mapped SocketAddr.
    /// Expects the response to correspond to the given transaction ID.
    pub fn parse_binding_response(
        buf: &[u8],
        expected_tx_id: &[u8; 12],
    ) -> Result<SocketAddr, StunError> {
        if buf.len() < 20 {
            return Err(StunError::InvalidResponse(format!(
                "Packet too short: {} bytes (min 20)",
                buf.len()
            )));
        }

        let msg_type = u16::from_be_bytes([buf[0], buf[1]]);
        let msg_len = u16::from_be_bytes([buf[2], buf[3]]) as usize;
        let cookie = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
        let tx_id = &buf[8..20];

        if msg_type != BINDING_RESPONSE {
            return Err(StunError::InvalidResponse(format!(
                "Expected Binding Response (0x0101), got 0x{:04X}",
                msg_type
            )));
        }

        if cookie != MAGIC_COOKIE {
            return Err(StunError::InvalidResponse(format!(
                "Invalid Magic Cookie: 0x{:08X}",
                cookie
            )));
        }

        if tx_id != expected_tx_id {
            return Err(StunError::InvalidResponse(
                "Transaction ID mismatch".to_string(),
            ));
        }

        if buf.len() < 20 + msg_len {
            return Err(StunError::InvalidResponse(format!(
                "Payload smaller than header declared: got {} bytes, expected {}",
                buf.len(),
                20 + msg_len
            )));
        }

        let mut offset = 20;
        let end = 20 + msg_len;
        let mut xor_mapped: Option<SocketAddr> = None;
        let mut mapped: Option<SocketAddr> = None;

        while offset + 4 <= end {
            let attr_type = u16::from_be_bytes([buf[offset], buf[offset + 1]]);
            let attr_len = u16::from_be_bytes([buf[offset + 2], buf[offset + 3]]) as usize;
            let val_start = offset + 4;
            let val_end = val_start + attr_len;

            if val_end > end {
                break;
            }

            let attr_val = &buf[val_start..val_end];

            if attr_type == ATTR_XOR_MAPPED_ADDRESS {
                if let Ok(addr) = Self::parse_xor_mapped_address(attr_val, expected_tx_id) {
                    xor_mapped = Some(addr);
                }
            } else if attr_type == ATTR_MAPPED_ADDRESS {
                if let Ok(addr) = Self::parse_mapped_address(attr_val) {
                    mapped = Some(addr);
                }
            }

            // STUN attributes are padded to 4-byte boundaries (RFC 5389 Section 15)
            let padded_len = (attr_len + 3) & !3;
            offset = val_start + padded_len;
        }

        if let Some(addr) = xor_mapped {
            Ok(addr)
        } else if let Some(addr) = mapped {
            Ok(addr)
        } else {
            Err(StunError::MappedAddressNotFound)
        }
    }

    fn parse_mapped_address(val: &[u8]) -> Result<SocketAddr, StunError> {
        if val.len() < 4 {
            return Err(StunError::InvalidResponse(
                "MAPPED-ADDRESS attribute too short".into(),
            ));
        }
        let family = val[1];
        let port = u16::from_be_bytes([val[2], val[3]]);

        match family {
            0x01 => {
                // IPv4
                if val.len() < 8 {
                    return Err(StunError::InvalidResponse(
                        "MAPPED-ADDRESS IPv4 value too short".into(),
                    ));
                }
                let ip = Ipv4Addr::new(val[4], val[5], val[6], val[7]);
                Ok(SocketAddr::new(IpAddr::V4(ip), port))
            }
            0x02 => {
                // IPv6
                if val.len() < 20 {
                    return Err(StunError::InvalidResponse(
                        "MAPPED-ADDRESS IPv6 value too short".into(),
                    ));
                }
                let mut octets = [0u8; 16];
                octets.copy_from_slice(&val[4..20]);
                let ip = Ipv6Addr::from(octets);
                Ok(SocketAddr::new(IpAddr::V6(ip), port))
            }
            other => Err(StunError::AddressFamilyNotSupported(other)),
        }
    }

    fn parse_xor_mapped_address(val: &[u8], tx_id: &[u8; 12]) -> Result<SocketAddr, StunError> {
        if val.len() < 4 {
            return Err(StunError::InvalidResponse(
                "XOR-MAPPED-ADDRESS attribute too short".into(),
            ));
        }
        let family = val[1];
        let raw_port = u16::from_be_bytes([val[2], val[3]]);
        let magic_16 = (MAGIC_COOKIE >> 16) as u16; // 0x2112
        let port = raw_port ^ magic_16;

        match family {
            0x01 => {
                // IPv4
                if val.len() < 8 {
                    return Err(StunError::InvalidResponse(
                        "XOR-MAPPED-ADDRESS IPv4 value too short".into(),
                    ));
                }
                let raw_ip = u32::from_be_bytes([val[4], val[5], val[6], val[7]]);
                let ip = Ipv4Addr::from(raw_ip ^ MAGIC_COOKIE);
                Ok(SocketAddr::new(IpAddr::V4(ip), port))
            }
            0x02 => {
                // IPv6
                if val.len() < 20 {
                    return Err(StunError::InvalidResponse(
                        "XOR-MAPPED-ADDRESS IPv6 value too short".into(),
                    ));
                }
                let mut xor_mask = [0u8; 16];
                xor_mask[0..4].copy_from_slice(&MAGIC_COOKIE.to_be_bytes());
                xor_mask[4..16].copy_from_slice(tx_id);

                let mut octets = [0u8; 16];
                for i in 0..16 {
                    octets[i] = val[4 + i] ^ xor_mask[i];
                }
                let ip = Ipv6Addr::from(octets);
                Ok(SocketAddr::new(IpAddr::V6(ip), port))
            }
            other => Err(StunError::AddressFamilyNotSupported(other)),
        }
    }
}

/// Queries a STUN server for the external reflexive address using a default 3.0s timeout.
pub async fn query_stun(server_addr: &str) -> Result<SocketAddr, StunError> {
    query_stun_with_timeout(server_addr, Duration::from_millis(3000)).await
}

/// Queries a STUN server with a specified timeout.
pub async fn query_stun_with_timeout(
    server_addr: &str,
    timeout_duration: Duration,
) -> Result<SocketAddr, StunError> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.connect(server_addr).await?;

    let mut tx_id = [0u8; 12];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut tx_id);

    let req = StunClient::build_binding_request(&tx_id);
    socket.send(&req).await?;

    let mut buf = [0u8; 1024];

    let n = match timeout(timeout_duration, socket.recv(&mut buf)).await {
        Ok(res) => res?,
        Err(_) => return Err(StunError::Timeout),
    };

    StunClient::parse_binding_response(&buf[..n], &tx_id)
}
