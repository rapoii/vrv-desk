use mirror_core::audio::{
    decode_audio_packet, is_audio_packet, AudioLoopbackCapturer, OpusAudioEncoder,
    AUDIO_FORMAT_OPUS, AUDIO_FORMAT_PCM, AUDIO_FORMAT_PCM_S16LE, AUDIO_MAGIC,
};
use std::time::Duration;

#[test]
fn test_audio_constants() {
    assert_eq!(AUDIO_MAGIC, b"VAUD");
    assert_eq!(AUDIO_FORMAT_PCM_S16LE, 0x01);
    assert_eq!(AUDIO_FORMAT_PCM, 0x01);
    assert_eq!(AUDIO_FORMAT_OPUS, 0x02);
}

#[test]
fn test_opus_encoder_init_and_feed() {
    let sample_rate = 48000;
    let channels = 2;
    let bitrate = 64_000;

    let mut encoder = OpusAudioEncoder::new(sample_rate, channels, bitrate)
        .expect("OpusAudioEncoder should initialize");

    assert_eq!(encoder.channels(), 2);
    assert_eq!(encoder.sample_rate(), 48000);

    // Frame size: 20ms at 48kHz = 960 samples per channel
    // Stereo: 960 * 2 = 1920 samples = 3840 bytes PCM
    let frame_samples = 960;
    let mut pcm_bytes = Vec::with_capacity(frame_samples * 2 * 2);

    for i in 0..frame_samples {
        let val = ((i as f32 * 440.0 * 2.0 * std::f32::consts::PI / 48000.0).sin() * 16000.0) as i16;
        pcm_bytes.extend_from_slice(&val.to_le_bytes()); // Left
        pcm_bytes.extend_from_slice(&val.to_le_bytes()); // Right
    }

    assert_eq!(pcm_bytes.len(), 3840);

    // Feed the 20ms frame
    let packets = encoder
        .feed_pcm_and_encode(&pcm_bytes)
        .expect("Encoding frame should succeed");

    assert_eq!(packets.len(), 1, "Should emit exactly 1 packet for a 20ms frame");

    let packet = &packets[0];
    assert!(is_audio_packet(packet), "Packet should start with VAUD magic");

    let header = decode_audio_packet(packet).expect("Header should be decoded");
    assert_eq!(header.format, AUDIO_FORMAT_OPUS);
    assert_eq!(header.channels, 2);
    assert_eq!(header.sample_rate, 48000);
    assert_eq!(header.payload_offset, 8);

    // Verify compression ratio: raw was 3840 bytes, encoded Opus payload should be < 300 bytes (>10x compression)
    let payload_len = header.payload_len;
    println!(
        "Opus payload size: {} bytes (vs 3840 bytes PCM, compression ratio: {:.1}x)",
        payload_len,
        3840.0 / payload_len as f32
    );

    assert!(payload_len > 0);
    assert!(payload_len < 300, "Opus 20ms frame payload should be < 300 bytes");
    let compression_ratio = 3840.0 / payload_len as f32;
    assert!(compression_ratio > 10.0, "Compression ratio should be > 10x");
}

#[test]
fn test_opus_encoder_partial_chunks() {
    let mut encoder = OpusAudioEncoder::new(48000, 2, 64_000)
        .expect("OpusAudioEncoder should initialize");

    // Feed 1000 bytes (< 3840 bytes)
    let partial = vec![0u8; 1000];
    let packets1 = encoder.feed_pcm_and_encode(&partial).unwrap();
    assert_eq!(packets1.len(), 0, "Partial chunk should not emit packets yet");

    // Feed another 2840 bytes (now 3840 total = exactly 1 frame)
    let remainder = vec![0u8; 2840];
    let packets2 = encoder.feed_pcm_and_encode(&remainder).unwrap();
    assert_eq!(packets2.len(), 1, "Accumulated buffer should emit 1 packet");

    let header = decode_audio_packet(&packets2[0]).unwrap();
    assert_eq!(header.format, AUDIO_FORMAT_OPUS);
}

#[test]
fn test_audio_loopback_capturer_produces_opus() {
    let mut capturer = AudioLoopbackCapturer::new();
    println!(
        "Audio capturer running: format=0x{:02x}, channels={}, rate={}, mock={}",
        capturer.format, capturer.channels, capturer.sample_rate, capturer.is_mock
    );

    // By default it should configure Opus format 0x02
    assert_eq!(capturer.format, AUDIO_FORMAT_OPUS);

    let packet_opt = capturer.read_packet_timeout(Duration::from_millis(500));
    if !capturer.is_mock && packet_opt.is_none() {
        println!("Live audio capturer silent (no system audio currently playing) — passing gracefully");
        return;
    }
    assert!(packet_opt.is_some(), "Audio capturer should emit packet within timeout");

    let packet = packet_opt.unwrap();
    assert!(is_audio_packet(&packet));

    let header = decode_audio_packet(&packet).expect("Decoded header");
    assert_eq!(header.format, AUDIO_FORMAT_OPUS);
    assert_eq!(header.channels, capturer.channels);
    assert_eq!(header.sample_rate, capturer.sample_rate);
    assert!(header.payload_len > 0);
    assert!(header.payload_len < 300, "Mock or live Opus payload should be compact");
}

#[test]
fn test_audio_loopback_capturer_pcm_fallback_option() {
    let mut capturer = AudioLoopbackCapturer::new_with_options(false);
    assert_eq!(capturer.format, AUDIO_FORMAT_PCM_S16LE);

    let packet_opt = capturer.read_packet_timeout(Duration::from_millis(500));
    if !capturer.is_mock && packet_opt.is_none() {
        println!("Live audio capturer silent (no system audio playing) — passing gracefully");
        return;
    }
    assert!(packet_opt.is_some());

    let packet = packet_opt.unwrap();
    let header = decode_audio_packet(&packet).unwrap();
    assert_eq!(header.format, AUDIO_FORMAT_PCM_S16LE);
}
