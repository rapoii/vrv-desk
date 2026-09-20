use mirror_core::video::{H264VideoEncoder, FRAME_TYPE_DELTA, FRAME_TYPE_IDR, VIDEO_MAGIC};

#[test]
fn test_h264_encoder_initialization() {
    let encoder = H264VideoEncoder::new(640, 360, 2_500_000, 60.0);
    assert!(encoder.is_ok(), "Encoder should initialize successfully");
}

#[test]
fn test_h264_encode_frame_and_header() {
    let mut encoder = H264VideoEncoder::new(640, 360, 2_500_000, 60.0).expect("Init failed");

    // Frame 1: Initial IDR Keyframe
    let rgb = vec![120u8; 640 * 360 * 3];
    let packet = encoder.encode_rgb(640, 360, &rgb).expect("Encode failed");

    assert!(packet.len() > 8, "Packet must have header + NAL payload");
    assert_eq!(&packet[0..4], VIDEO_MAGIC, "Packet must start with VH24");
    assert_eq!(
        packet[4], FRAME_TYPE_IDR,
        "First packet must be IDR keyframe"
    );

    // Verify Annex-B NAL start code (0x00, 0x00, 0x00, 0x01)
    let nal_payload = &packet[8..];
    assert!(
        nal_payload.starts_with(&[0, 0, 0, 1]),
        "Payload must start with Annex-B prefix"
    );

    // Frame 2: Identical / static frame (should produce P-frame and be significantly smaller)
    let packet2 = encoder
        .encode_rgb(640, 360, &rgb)
        .expect("Encode frame 2 failed");
    assert_eq!(&packet2[0..4], VIDEO_MAGIC);
    assert_eq!(packet2[4], FRAME_TYPE_DELTA, "Second frame should be delta");
    assert!(
        packet2.len() <= packet.len(),
        "P-frame ({} bytes) should be smaller or equal to IDR keyframe ({} bytes)",
        packet2.len(),
        packet.len()
    );
}

#[test]
fn test_h264_encode_bgra() {
    let mut encoder = H264VideoEncoder::new(320, 240, 1_500_000, 60.0).expect("Init failed");
    // BGRA format (e.g. from DXGI capture)
    let bgra = vec![255u8; 320 * 240 * 4];
    let packet = encoder
        .encode_bgra(320, 240, &bgra)
        .expect("BGRA encode failed");
    assert_eq!(&packet[0..4], VIDEO_MAGIC);
    assert!(packet.len() > 8);
}
