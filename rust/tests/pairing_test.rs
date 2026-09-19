use mirror_core::pairing::{PairingClient, PairingHost};

#[test]
fn test_pin_handshake_successful_key_exchange() {
    let mut host = PairingHost::new();
    let pin = host.generate_dynamic_pin(120); // 120 seconds TTL
    assert_eq!(pin.len(), 6);

    let client = PairingClient::new();
    let client_hello = client.create_hello(&pin);

    let host_response = host
        .process_hello(&client_hello, &pin)
        .expect("PIN must match");
    let client_key = client
        .finalize(&host_response)
        .expect("Handshake should complete");

    assert_eq!(host.session_key().unwrap(), client_key);
}

#[test]
fn test_pin_handshake_invalid_pin_fails() {
    let mut host = PairingHost::new();
    let pin = host.generate_dynamic_pin(120);

    let client = PairingClient::new();
    let client_hello = client.create_hello("000000");

    let result = host.process_hello(&client_hello, &pin);
    assert!(result.is_err(), "Handshake with wrong PIN must be rejected");
}
