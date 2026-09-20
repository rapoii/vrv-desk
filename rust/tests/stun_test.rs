use mirror_core::stun::{
    self, StunClient, ATTR_MAPPED_ADDRESS, ATTR_XOR_MAPPED_ADDRESS, BINDING_RESPONSE,
    DEFAULT_STUN_SERVER, MAGIC_COOKIE,
};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[test]
fn test_build_binding_request_structure() {
    let tx_id = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    let req = StunClient::build_binding_request(&tx_id);

    assert_eq!(req.len(), 20);
    // Message Type: 0x0001
    assert_eq!(&req[0..2], &[0x00, 0x01]);
    // Message Length: 0x0000
    assert_eq!(&req[2..4], &[0x00, 0x00]);
    // Magic Cookie: 0x2112A442
    assert_eq!(&req[4..8], &[0x21, 0x12, 0xA4, 0x42]);
    // Transaction ID
    assert_eq!(&req[8..20], &tx_id);
}

#[test]
fn test_parse_xor_mapped_address_ipv4() {
    let tx_id = [0xAA; 12];
    let mut response = Vec::new();

    // STUN Header (20 bytes)
    response.extend_from_slice(&BINDING_RESPONSE.to_be_bytes()); // 0x0101
    let attr_len: u16 = 8; // XOR-MAPPED-ADDRESS value length for IPv4
    let total_attr_field_len: u16 = 4 + attr_len; // 12 bytes
    response.extend_from_slice(&total_attr_field_len.to_be_bytes());
    response.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());
    response.extend_from_slice(&tx_id);

    // Attribute Header
    response.extend_from_slice(&ATTR_XOR_MAPPED_ADDRESS.to_be_bytes()); // 0x0020
    response.extend_from_slice(&attr_len.to_be_bytes()); // length: 8

    // Attribute Value:
    // Reserved (0x00), Family (0x01 = IPv4)
    response.push(0x00);
    response.push(0x01);

    // Target IP: 203.0.113.195 => 0xCB0071C3
    // Target Port: 54320 => 0xD430
    let target_port: u16 = 54320;
    let xor_port = target_port ^ ((MAGIC_COOKIE >> 16) as u16);
    response.extend_from_slice(&xor_port.to_be_bytes());

    let target_ip_u32 = u32::from(Ipv4Addr::new(203, 0, 113, 195));
    let xor_ip_u32 = target_ip_u32 ^ MAGIC_COOKIE;
    response.extend_from_slice(&xor_ip_u32.to_be_bytes());

    let parsed = StunClient::parse_binding_response(&response, &tx_id)
        .expect("Should parse valid XOR address");
    assert_eq!(
        parsed,
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 195)), 54320)
    );
}

#[test]
fn test_parse_mapped_address_fallback_ipv4() {
    let tx_id = [0xBB; 12];
    let mut response = Vec::new();

    // STUN Header (20 bytes)
    response.extend_from_slice(&BINDING_RESPONSE.to_be_bytes());
    let attr_len: u16 = 8;
    let total_attr_field_len: u16 = 4 + attr_len;
    response.extend_from_slice(&total_attr_field_len.to_be_bytes());
    response.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());
    response.extend_from_slice(&tx_id);

    // Attribute Header
    response.extend_from_slice(&ATTR_MAPPED_ADDRESS.to_be_bytes()); // 0x0001
    response.extend_from_slice(&attr_len.to_be_bytes());

    // Attribute Value (unmasked)
    response.push(0x00);
    response.push(0x01); // IPv4
    response.extend_from_slice(&12345u16.to_be_bytes());
    response.extend_from_slice(&[198, 51, 100, 1]);

    let parsed = StunClient::parse_binding_response(&response, &tx_id)
        .expect("Should parse valid MAPPED address");
    assert_eq!(
        parsed,
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(198, 51, 100, 1)), 12345)
    );
}

#[test]
fn test_transaction_id_mismatch() {
    let tx_id = [0x01; 12];
    let diff_tx_id = [0x02; 12];

    let mut response = Vec::new();
    response.extend_from_slice(&BINDING_RESPONSE.to_be_bytes());
    response.extend_from_slice(&0u16.to_be_bytes());
    response.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());
    response.extend_from_slice(&diff_tx_id);

    let res = StunClient::parse_binding_response(&response, &tx_id);
    assert!(res.is_err());
}

#[tokio::test]
async fn test_live_stun_query() {
    println!("Connecting to STUN server: {}", DEFAULT_STUN_SERVER);
    match stun::query_stun(DEFAULT_STUN_SERVER).await {
        Ok(addr) => {
            println!("Discovered public reflexive address: {}", addr);
            assert!(addr.port() > 0);
            match addr.ip() {
                IpAddr::V4(ipv4) => {
                    assert!(!ipv4.is_unspecified());
                }
                IpAddr::V6(ipv6) => {
                    assert!(!ipv6.is_unspecified());
                }
            }
        }
        Err(e) => {
            // If offline or network blocks UDP 19302, log warning
            eprintln!(
                "Warning: Live STUN query failed (possibly offline or blocked UDP): {}",
                e
            );
        }
    }
}
