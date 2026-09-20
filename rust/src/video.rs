use openh264::encoder::{
    BitRate, Encoder, EncoderConfig, FrameRate, RateControlMode, UsageType,
};
use openh264::formats::{RgbSliceU8, YUVBuffer};
use openh264::OpenH264API;

pub const VIDEO_MAGIC: &[u8; 4] = b"VH24";
pub const FRAME_TYPE_IDR: u8 = 0x01;
pub const FRAME_TYPE_DELTA: u8 = 0x02;

pub struct H264VideoEncoder {
    encoder: Encoder,
    width: u32,
    height: u32,
    bitrate: u32,
    framerate: f32,
    frame_seq: u16,
    frames_since_idr: u32,
    keyframe_interval: u32,
}

impl H264VideoEncoder {
    /// Create a new H264 Screen Content RealTime encoder
    pub fn new(
        width: u32,
        height: u32,
        bitrate: u32,
        framerate: f32,
    ) -> Result<Self, String> {
        let config = EncoderConfig::new()
            .bitrate(BitRate::from_bps(bitrate))
            .max_frame_rate(FrameRate::from_hz(framerate))
            .usage_type(UsageType::ScreenContentRealTime)
            .rate_control_mode(RateControlMode::Bitrate);

        let api = OpenH264API::from_source();
        let encoder = Encoder::with_api_config(api, config)
            .map_err(|e| format!("Failed to create OpenH264 encoder: {:?}", e))?;

        Ok(Self {
            encoder,
            width,
            height,
            bitrate,
            framerate,
            frame_seq: 0,
            frames_since_idr: 0,
            keyframe_interval: 60,
        })
    }

    /// Helper to detect if a bitstream contains an SPS, PPS, or IDR keyframe
    fn is_keyframe(data: &[u8]) -> bool {
        let mut i = 0;
        while i + 4 < data.len() {
            if data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 0 && data[i + 3] == 1 {
                let nal_type = data[i + 4] & 0x1F;
                if nal_type == 5 || nal_type == 7 || nal_type == 8 {
                    return true;
                }
                i += 4;
            } else if data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1 {
                let nal_type = data[i + 3] & 0x1F;
                if nal_type == 5 || nal_type == 7 || nal_type == 8 {
                    return true;
                }
                i += 3;
            } else {
                i += 1;
            }
        }
        false
    }

    /// Wrap raw NAL bitstream in 8-byte VH24 packet header:
    /// [0..4]  = b"VH24"
    /// [4]     = frame_type (0x01 = IDR, 0x02 = DELTA)
    /// [5]     = flags (0x00)
    /// [6..8]  = frame sequence number (u16 Big-Endian)
    fn package_vh24(&mut self, nal_bytes: &[u8]) -> Vec<u8> {
        let is_idr = Self::is_keyframe(nal_bytes) || self.frame_seq == 0;
        let frame_type = if is_idr {
            self.frames_since_idr = 0;
            FRAME_TYPE_IDR
        } else {
            self.frames_since_idr += 1;
            FRAME_TYPE_DELTA
        };

        let mut packet = Vec::with_capacity(8 + nal_bytes.len());
        packet.extend_from_slice(VIDEO_MAGIC);
        packet.push(frame_type);
        packet.push(0x00); // flags
        packet.extend_from_slice(&self.frame_seq.to_be_bytes());
        packet.extend_from_slice(nal_bytes);

        self.frame_seq = self.frame_seq.wrapping_add(1);
        packet
    }

    /// Encode an RGB8 frame (width * height * 3 bytes)
    pub fn encode_rgb(
        &mut self,
        width: u32,
        height: u32,
        rgb_data: &[u8],
    ) -> Result<Vec<u8>, String> {
        if width != self.width || height != self.height {
            // Reinitialize encoder if resolution dynamically changes
            let config = EncoderConfig::new()
                .bitrate(BitRate::from_bps(self.bitrate))
                .max_frame_rate(FrameRate::from_hz(self.framerate))
                .usage_type(UsageType::ScreenContentRealTime)
                .rate_control_mode(RateControlMode::Bitrate);
            let api = OpenH264API::from_source();
            self.encoder = Encoder::with_api_config(api, config)
                .map_err(|e| format!("Re-init OpenH264 encoder failed: {:?}", e))?;
            self.width = width;
            self.height = height;
        }

        let rgb_source = RgbSliceU8::new(rgb_data, (width as usize, height as usize));
        let yuv = YUVBuffer::from_rgb_source(rgb_source);
        let bitstream = self
            .encoder
            .encode(&yuv)
            .map_err(|e| format!("OpenH264 encode error: {:?}", e))?;

        let nal_bytes = bitstream.to_vec();
        Ok(self.package_vh24(&nal_bytes))
    }

    /// Encode a BGRA frame (width * height * 4 bytes) directly from DXGI capture
    pub fn encode_bgra(
        &mut self,
        width: u32,
        height: u32,
        bgra_data: &[u8],
    ) -> Result<Vec<u8>, String> {
        let pixel_count = (width * height) as usize;
        let mut rgb = vec![0u8; pixel_count * 3];

        for (bgra_px, rgb_px) in bgra_data.chunks_exact(4).zip(rgb.chunks_exact_mut(3)) {
            rgb_px[0] = bgra_px[2]; // R
            rgb_px[1] = bgra_px[1]; // G
            rgb_px[2] = bgra_px[0]; // B
        }

        self.encode_rgb(width, height, &rgb)
    }
}

unsafe impl Send for VideoEncoder {}

pub enum VideoEncoder {
    #[cfg(windows)]
    HardwareMft(crate::mft_encoder::MftH264Encoder),
    SoftwareOpenH264(H264VideoEncoder),
}

impl VideoEncoder {
    pub fn new(width: u32, height: u32, bitrate: u32, framerate: f32) -> Result<Self, String> {
        #[cfg(windows)]
        {
            match crate::mft_encoder::MftH264Encoder::new(width, height, bitrate, framerate) {
                Ok(mft) => {
                    println!("🚀 Video compression: Hardware MFT (NVENC / Intel QSV)");
                    return Ok(Self::HardwareMft(mft));
                }
                Err(e) => {
                    eprintln!(
                        "⚠️ Hardware MFT encoder unavailable ({}), falling back to OpenH264 software encoder",
                        e
                    );
                }
            }
        }

        let sw = H264VideoEncoder::new(width, height, bitrate, framerate)?;
        println!("🚀 Video compression: Cisco OpenH264 ScreenContentRealTime");
        Ok(Self::SoftwareOpenH264(sw))
    }

    pub fn encode_bgra(
        &mut self,
        width: u32,
        height: u32,
        bgra_data: &[u8],
    ) -> Result<Vec<u8>, String> {
        match self {
            #[cfg(windows)]
            Self::HardwareMft(ref mut mft) => {
                match mft.encode_bgra(width, height, bgra_data) {
                    Ok(bytes) => Ok(bytes),
                    Err(_) => {
                        // Fallback to software encoding if a single frame fails
                        let mut sw = H264VideoEncoder::new(width, height, 2_500_000, 60.0)?;
                        let bytes = sw.encode_bgra(width, height, bgra_data)?;
                        *self = Self::SoftwareOpenH264(sw);
                        Ok(bytes)
                    }
                }
            }
            Self::SoftwareOpenH264(ref mut sw) => sw.encode_bgra(width, height, bgra_data),
        }
    }
}

