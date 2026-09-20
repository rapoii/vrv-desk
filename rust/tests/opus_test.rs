use opus::{Application, Channels, Decoder, Encoder};

#[test]
fn test_opus_encode_decode_roundtrip() {
    let sample_rate = 48000;
    let channels = Channels::Stereo;
    let mut encoder =
        Encoder::new(sample_rate, channels, Application::Audio).expect("create encoder");
    encoder
        .set_bitrate(opus::Bitrate::Bits(64_000))
        .expect("set bitrate");
    let mut decoder = Decoder::new(sample_rate, channels).expect("create decoder");

    // 20ms frame = 960 samples per channel = 1920 i16 values
    let frame_size = 960;
    let mut pcm_in = Vec::with_capacity(frame_size * 2);
    for i in 0..frame_size {
        // Generate a simple 440 Hz sine wave tone
        let val =
            ((i as f32 * 440.0 * 2.0 * std::f32::consts::PI / 48000.0).sin() * 16000.0) as i16;
        pcm_in.push(val); // L
        pcm_in.push(val); // R
    }

    let mut opus_out = vec![0u8; 1000];
    let encoded_len = encoder.encode(&pcm_in, &mut opus_out).expect("encode opus");

    println!(
        "PCM size: {} bytes, Opus encoded size: {} bytes (compression ratio: {:.1}x)",
        pcm_in.len() * 2,
        encoded_len,
        (pcm_in.len() * 2) as f32 / encoded_len as f32
    );

    assert!(encoded_len > 0);
    assert!(
        encoded_len < 300,
        "Opus 20ms packet should be compact (< 300 bytes)"
    );

    // Decode back
    let mut pcm_out = vec![0i16; frame_size * 2];
    let decoded_samples = decoder
        .decode(&opus_out[..encoded_len], &mut pcm_out, false)
        .expect("decode opus");

    assert_eq!(decoded_samples, frame_size);
}
