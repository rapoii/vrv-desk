use mirror_core::identity::DeviceIdentity;

#[test]
fn test_device_identity_generation_and_id_format() {
    let identity = DeviceIdentity::generate();
    let id = identity.device_id();
    
    // Must be exactly 6 digits
    assert_eq!(id.len(), 6);
    assert!(id.chars().all(|c| c.is_ascii_digit()));
    
    // Must be deterministic from public key
    assert_eq!(identity.device_id(), id);
    
    // Different identities should yield different IDs with high probability
    let other = DeviceIdentity::generate();
    assert_ne!(identity.public_key_bytes(), other.public_key_bytes());
}
