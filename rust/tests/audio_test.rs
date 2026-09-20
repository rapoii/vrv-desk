use mirror_core::audio::{
    decode_audio_packet, encode_audio_packet, is_audio_packet, AudioLoopbackCapturer,
    AUDIO_FORMAT_PCM_S16LE, AUDIO_MAGIC,
};
use std::time::Duration;

#[test]
fn test_audio_magic_constants() {
    assert_eq!(AUDIO_MAGIC, b"VAUD");
    assert_eq!(AUDIO_FORMAT_PCM_S16LE, 0x01);
}

#[test]
fn test_encode_decode_audio_packet() {
    let dummy_pcm: Vec<u8> = (0..100).map(|i| (i * 2) as u8).collect();
    let format = AUDIO_FORMAT_PCM_S16LE;
    let channels = 2;
    let sample_rate = 48000;

    let packet = encode_audio_packet(format, channels, sample_rate, &dummy_pcm);

    assert_eq!(packet.len(), 8 + dummy_pcm.len());
    assert_eq!(&packet[0..4], b"VAUD");
    assert_eq!(packet[4], format);
    assert_eq!(packet[5], channels);
    assert_eq!(
        u16::from_le_bytes([packet[6], packet[7]]),
        sample_rate
    );
    assert_eq!(&packet[8..], &dummy_pcm[..]);

    let decoded = decode_audio_packet(&packet).expect("Should decode successfully");
    assert_eq!(decoded.format, format);
    assert_eq!(decoded.channels, channels);
    assert_eq!(decoded.sample_rate, sample_rate);
    assert_eq!(decoded.payload_offset, 8);
    assert_eq!(decoded.payload_len, dummy_pcm.len());
    assert_eq!(&packet[decoded.payload_offset..], &dummy_pcm[..]);
}

#[test]
fn test_decode_invalid_packets() {
    // Too short (< 8 bytes)
    assert!(decode_audio_packet(b"VAUD").is_none());
    assert!(decode_audio_packet(b"VAUD123").is_none());

    // Wrong magic
    let bad_magic = b"NOTV\x01\x02\x80\xbb1234";
    assert!(decode_audio_packet(bad_magic).is_none());
}

#[test]
fn test_is_audio_packet() {
    let audio_pkt = encode_audio_packet(0x01, 2, 48000, &[1, 2, 3, 4]);
    assert!(is_audio_packet(&audio_pkt));

    // JPEG header: [0xFF, 0xD8, 0xFF, ...]
    let jpeg_header = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
    assert!(!is_audio_packet(&jpeg_header));

    // Arbitrary short slice
    assert!(!is_audio_packet(&[0x56, 0x41]));
    // Empty slice
    assert!(!is_audio_packet(&[]));
}

#[test]
fn test_mock_loopback_capturer() {
    let mut capturer = AudioLoopbackCapturer::new_mock(2, 44100);
    assert!(capturer.is_mock);
    assert_eq!(capturer.channels, 2);
    assert_eq!(capturer.sample_rate, 44100);

    // Read a packet within a reasonable timeout
    let packet = capturer.read_packet_timeout(Duration::from_millis(500));
    assert!(packet.is_some(), "Mock capturer should emit audio packets");
    let packet_bytes = packet.unwrap();
    assert!(is_audio_packet(&packet_bytes));

    let header = decode_audio_packet(&packet_bytes).expect("Mock packet should decode");
    assert_eq!(header.format, AUDIO_FORMAT_PCM_S16LE);
    assert_eq!(header.channels, 2);
    assert_eq!(header.sample_rate, 44100);
    assert!(header.payload_len > 0);
}

#[test]
fn test_live_or_fallback_capturer_initialization() {
    let mut capturer = AudioLoopbackCapturer::new();
    println!(
        "Audio capturer initialized: mock={}, channels={}, rate={}",
        capturer.is_mock, capturer.channels, capturer.sample_rate
    );

    assert!(capturer.channels >= 1);
    assert!(capturer.sample_rate >= 8000);

    // Give it a brief window to capture a packet (either live or mock)
    let packet = capturer.read_packet_timeout(Duration::from_millis(500));
    if !capturer.is_mock && packet.is_none() {
        println!("Live audio capturer silent (no sound currently playing) — passing gracefully");
        return;
    }
    assert!(packet.is_some(), "Audio capturer should produce packets");
    let p = packet.unwrap();
    assert!(is_audio_packet(&p));
    let header = decode_audio_packet(&p).expect("Packet should be valid VAUD");
    assert_eq!(header.format, capturer.format);
    assert_eq!(header.channels, capturer.channels);
    assert_eq!(header.sample_rate, capturer.sample_rate);
}
