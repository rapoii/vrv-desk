use mirror_core::transport::SecureTransportSession;

#[test]
fn test_e2ee_encrypt_decrypt_roundtrip() {
    let key = [0x42u8; 32];
    let mut sender = SecureTransportSession::new(&key);
    let mut receiver = SecureTransportSession::new(&key);

    let plaintext = b"Hello Secure Screen Mirroring E2EE!";
    let ciphertext = sender.encrypt(plaintext).expect("Encryption failed");

    assert!(SecureTransportSession::is_e2ee_packet(&ciphertext));
    assert_ne!(&ciphertext[12..], plaintext);

    let decrypted = receiver.decrypt(&ciphertext).expect("Decryption failed");
    assert_eq!(&decrypted, plaintext);
}

#[test]
fn test_e2ee_tamper_detection() {
    let key = [0x77u8; 32];
    let mut sender = SecureTransportSession::new(&key);
    let mut receiver = SecureTransportSession::new(&key);

    let plaintext = b"Sensitive keyboard password keystroke";
    let mut ciphertext = sender.encrypt(plaintext).expect("Encryption failed");

    // Tamper with one byte in the ciphertext payload
    let last_idx = ciphertext.len() - 1;
    ciphertext[last_idx] ^= 0x01;

    let res = receiver.decrypt(&ciphertext);
    assert!(res.is_err(), "Tampered packet MUST fail authentication");
}

#[test]
fn test_e2ee_derive_from_token() {
    let token = "session_token_xyz_123456";
    let mut s1 = SecureTransportSession::from_token(token);
    let mut s2 = SecureTransportSession::from_token(token);

    let data = b"Test token derived encryption";
    let enc = s1.encrypt(data).unwrap();
    let dec = s2.decrypt(&enc).unwrap();
    assert_eq!(dec, data);
}
