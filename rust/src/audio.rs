//! Audio loopback capture and VAUD packet framing module.
//!
//! Provides Windows WASAPI loopback capture converting output mix audio
//! to 16-bit PCM little-endian frames packaged into binary `VAUD` packets,
//! along with mock fallback support for non-Windows platforms or systems without audio devices.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[cfg(windows)]
use windows::core::GUID;
#[cfg(windows)]
use windows::Win32::Media::Audio::*;
#[cfg(windows)]
use windows::Win32::System::Com::*;

/// Magic header bytes for audio packets: ASCII "VAUD"
pub const AUDIO_MAGIC: &[u8; 4] = b"VAUD";

/// Standard format tag for raw signed 16-bit PCM little-endian
pub const AUDIO_FORMAT_PCM_S16LE: u8 = 0x01;

/// Audio packet header information parsed from a `VAUD` packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioPacketHeader {
    /// Audio format tag (e.g. 0x01 = PCM S16LE)
    pub format: u8,
    /// Number of channels (e.g. 1 = mono, 2 = stereo)
    pub channels: u8,
    /// Sample rate in Hz (e.g. 44100, 48000)
    pub sample_rate: u16,
    /// Offset where the raw audio payload starts (typically 8)
    pub payload_offset: usize,
    /// Length of the raw PCM payload in bytes
    pub payload_len: usize,
}

/// Checks whether a binary packet starts with the VAUD magic bytes.
pub fn is_audio_packet(packet: &[u8]) -> bool {
    packet.len() >= 4 && &packet[0..4] == AUDIO_MAGIC
}

/// Encodes raw PCM audio data into a binary VAUD packet.
///
/// Header structure (8 bytes total):
/// - Byte 0..4: `b"VAUD"` (`0x56, 0x41, 0x55, 0x44`)
/// - Byte 4: Format tag (e.g. `0x01` for PCM S16LE)
/// - Byte 5: Channels (e.g. `0x02` for stereo)
/// - Byte 6..8: Sample rate as `u16` Little-Endian (e.g. `48000`)
/// - Byte 8..: Raw PCM 16-bit signed LE bytes
pub fn encode_audio_packet(format: u8, channels: u8, sample_rate: u16, pcm: &[u8]) -> Vec<u8> {
    let mut packet = Vec::with_capacity(8 + pcm.len());
    packet.extend_from_slice(AUDIO_MAGIC);
    packet.push(format);
    packet.push(channels);
    packet.extend_from_slice(&sample_rate.to_le_bytes());
    packet.extend_from_slice(pcm);
    packet
}

/// Decodes and validates a VAUD packet header.
///
/// Returns `Some(AudioPacketHeader)` if valid, or `None` if the packet is too short
/// or doesn't match the `VAUD` magic identifier.
pub fn decode_audio_packet(packet: &[u8]) -> Option<AudioPacketHeader> {
    if packet.len() < 8 {
        return None;
    }
    if &packet[0..4] != AUDIO_MAGIC {
        return None;
    }

    let format = packet[4];
    let channels = packet[5];
    let sample_rate = u16::from_le_bytes([packet[6], packet[7]]);
    let payload_offset = 8;
    let payload_len = packet.len() - payload_offset;

    Some(AudioPacketHeader {
        format,
        channels,
        sample_rate,
        payload_offset,
        payload_len,
    })
}

/// Capturer for loopback system audio.
pub struct AudioLoopbackCapturer {
    receiver: std::sync::mpsc::Receiver<Vec<u8>>,
    is_running: Arc<AtomicBool>,
    join_handle: Option<std::thread::JoinHandle<()>>,
    pub channels: u8,
    pub sample_rate: u16,
    pub is_mock: bool,
}

impl AudioLoopbackCapturer {
    /// Creates and starts a new audio loopback capturer.
    ///
    /// On Windows, attempts WASAPI loopback capture on the default render device.
    /// If device acquisition fails, or on non-Windows platforms, falls back to a silent mock stream.
    pub fn new() -> Self {
        #[cfg(windows)]
        {
            match Self::try_init_wasapi() {
                Ok(capturer) => capturer,
                Err(err) => {
                    eprintln!(
                        "WASAPI loopback capture unavailable ({}), using fallback mock capturer",
                        err
                    );
                    Self::new_mock(2, 48000)
                }
            }
        }

        #[cfg(not(windows))]
        {
            Self::new_mock(2, 48000)
        }
    }

    /// Creates a mock silent audio capturer (useful for tests or fallback).
    pub fn new_mock(channels: u8, sample_rate: u16) -> Self {
        let (sender, receiver) = std::sync::mpsc::sync_channel::<Vec<u8>>(32);
        let is_running = Arc::new(AtomicBool::new(true));
        let is_running_clone = is_running.clone();

        let join_handle = std::thread::Builder::new()
            .name("audio_loopback_mock".to_string())
            .spawn(move || {
                // Generate ~20ms frames: sample_rate * 20 / 1000 samples per channel
                let samples_per_frame = (sample_rate as usize * 20) / 1000;
                let bytes_per_frame = samples_per_frame * (channels as usize) * 2;
                let silent_pcm = vec![0u8; bytes_per_frame];

                while is_running_clone.load(Ordering::Relaxed) {
                    let packet = encode_audio_packet(
                        AUDIO_FORMAT_PCM_S16LE,
                        channels,
                        sample_rate,
                        &silent_pcm,
                    );
                    if sender.send(packet).is_err() {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
            })
            .ok();

        Self {
            receiver,
            is_running,
            join_handle,
            channels,
            sample_rate,
            is_mock: true,
        }
    }

    /// Attempts to read the next captured VAUD packet non-blockingly.
    pub fn read_packet(&mut self) -> Option<Vec<u8>> {
        self.receiver.try_recv().ok()
    }

    /// Synchronously waits and reads the next captured VAUD packet with a timeout.
    pub fn read_packet_timeout(&mut self, timeout: Duration) -> Option<Vec<u8>> {
        self.receiver.recv_timeout(timeout).ok()
    }

    #[cfg(windows)]
    fn try_init_wasapi() -> Result<Self, String> {
        let (sender, receiver) = std::sync::mpsc::sync_channel::<Vec<u8>>(64);
        let is_running = Arc::new(AtomicBool::new(true));
        let is_running_clone = is_running.clone();

        let (init_tx, init_rx) = std::sync::mpsc::channel::<Result<(u8, u16), String>>();

        let join_handle = std::thread::Builder::new()
            .name("audio_loopback_wasapi".to_string())
            .spawn(move || {
                unsafe {
                    let coinit_res = CoInitializeEx(None, COINIT_MULTITHREADED);
                    let should_uninit = coinit_res.is_ok();

                    let res = run_wasapi_capture_loop(sender, is_running_clone, &init_tx);
                    if let Err(e) = res {
                        let _ = init_tx.send(Err(e));
                    }

                    if should_uninit {
                        CoUninitialize();
                    }
                }
            })
            .map_err(|e| format!("Failed to spawn audio capture thread: {}", e))?;

        let (channels, sample_rate) = init_rx
            .recv_timeout(Duration::from_secs(3))
            .map_err(|e| format!("Audio capturer initialization timeout: {}", e))?
            .map_err(|e| format!("Audio capturer initialization failed: {}", e))?;

        Ok(Self {
            receiver,
            is_running,
            join_handle: Some(join_handle),
            channels,
            sample_rate,
            is_mock: false,
        })
    }
}

impl Default for AudioLoopbackCapturer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AudioLoopbackCapturer {
    fn drop(&mut self) {
        self.is_running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(windows)]
unsafe fn run_wasapi_capture_loop(
    sender: std::sync::mpsc::SyncSender<Vec<u8>>,
    is_running: Arc<AtomicBool>,
    init_tx: &std::sync::mpsc::Sender<Result<(u8, u16), String>>,
) -> Result<(), String> {
    let enumerator: IMMDeviceEnumerator =
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
            .map_err(|e| format!("Failed to create MMDeviceEnumerator: {:?}", e))?;

    let device = enumerator
        .GetDefaultAudioEndpoint(eRender, eConsole)
        .map_err(|e| format!("Failed to get default audio render endpoint: {:?}", e))?;

    let client: IAudioClient = device
        .Activate(CLSCTX_ALL, None)
        .map_err(|e| format!("Failed to activate IAudioClient: {:?}", e))?;

    let pwfx = client
        .GetMixFormat()
        .map_err(|e| format!("Failed to get mix format: {:?}", e))?;

    let format_tag = (*pwfx).wFormatTag;
    let n_channels = (*pwfx).nChannels;
    let samples_per_sec = (*pwfx).nSamplesPerSec;
    let bits_per_sample = (*pwfx).wBitsPerSample;
    let cb_size = (*pwfx).cbSize;

    // Determine subformat if extensible
    let is_float = if format_tag == 3 {
        // WAVE_FORMAT_IEEE_FLOAT
        true
    } else if format_tag == 65534 && cb_size >= 22 {
        // WAVE_FORMAT_EXTENSIBLE
        let p_ext = pwfx as *const WAVEFORMATEXTENSIBLE;
        let subformat = (*p_ext).SubFormat;
        // KSDATAFORMAT_SUBTYPE_IEEE_FLOAT: 00000003-0000-0010-8000-00AA00389B71
        subformat == GUID::from_u128(0x00000003_0000_0010_8000_00aa00389b71)
    } else {
        false
    };

    let is_pcm16 = if format_tag == 1 && bits_per_sample == 16 {
        true
    } else if format_tag == 65534 && bits_per_sample == 16 && cb_size >= 22 {
        let p_ext = pwfx as *const WAVEFORMATEXTENSIBLE;
        let subformat = (*p_ext).SubFormat;
        // KSDATAFORMAT_SUBTYPE_PCM: 00000001-0000-0010-8000-00AA00389B71
        subformat == GUID::from_u128(0x00000001_0000_0010_8000_00aa00389b71)
    } else {
        false
    };

    if !is_float && !is_pcm16 {
        CoTaskMemFree(Some(pwfx as *const _ as *const _));
        return Err(format!(
            "Unsupported WASAPI mix format: tag={}, bits={}, channels={}",
            format_tag, bits_per_sample, n_channels
        ));
    }

    let channels = n_channels as u8;
    let sample_rate = samples_per_sec as u16;

    // Buffer duration 100ms in 100ns units
    let hns_buffer_duration: i64 = 100 * 10000;
    if let Err(e) = client.Initialize(
        AUDCLNT_SHAREMODE_SHARED,
        AUDCLNT_STREAMFLAGS_LOOPBACK,
        hns_buffer_duration,
        0,
        pwfx,
        None,
    ) {
        CoTaskMemFree(Some(pwfx as *const _ as *const _));
        return Err(format!("IAudioClient::Initialize failed: {:?}", e));
    }

    CoTaskMemFree(Some(pwfx as *const _ as *const _));

    let capture_client: IAudioCaptureClient = client
        .GetService()
        .map_err(|e| format!("IAudioClient::GetService failed: {:?}", e))?;

    client
        .Start()
        .map_err(|e| format!("IAudioClient::Start failed: {:?}", e))?;

    // Signal initialization success to parent thread
    let _ = init_tx.send(Ok((channels, sample_rate)));

    let mut pcm_out_buffer: Vec<u8> = Vec::with_capacity(8192);

    while is_running.load(Ordering::Relaxed) {
        let mut packet_size = match capture_client.GetNextPacketSize() {
            Ok(s) => s,
            Err(_) => break,
        };

        if packet_size == 0 {
            std::thread::sleep(Duration::from_millis(10));
            continue;
        }

        while packet_size > 0 && is_running.load(Ordering::Relaxed) {
            let mut data_ptr: *mut u8 = std::ptr::null_mut();
            let mut num_frames: u32 = 0;
            let mut flags: u32 = 0;

            let get_res = capture_client.GetBuffer(
                &mut data_ptr,
                &mut num_frames,
                &mut flags,
                None,
                None,
            );

            if get_res.is_err() {
                break;
            }

            if num_frames > 0 && !data_ptr.is_null() {
                let total_samples = (num_frames as usize) * (channels as usize);
                let pcm_byte_len = total_samples * 2;
                pcm_out_buffer.clear();
                pcm_out_buffer.reserve(pcm_byte_len);

                let is_silent = (flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32) != 0;

                if is_silent {
                    pcm_out_buffer.resize(pcm_byte_len, 0);
                } else if is_float {
                    let float_slice =
                        std::slice::from_raw_parts(data_ptr as *const f32, total_samples);
                    for &f in float_slice {
                        let sample = (f.clamp(-1.0, 1.0) * 32767.0) as i16;
                        pcm_out_buffer.extend_from_slice(&sample.to_le_bytes());
                    }
                } else if is_pcm16 {
                    let raw_bytes =
                        std::slice::from_raw_parts(data_ptr, pcm_byte_len);
                    pcm_out_buffer.extend_from_slice(raw_bytes);
                }

                if !pcm_out_buffer.is_empty() {
                    let packet = encode_audio_packet(
                        AUDIO_FORMAT_PCM_S16LE,
                        channels,
                        sample_rate,
                        &pcm_out_buffer,
                    );
                    let _ = sender.try_send(packet);
                }
            }

            let _ = capture_client.ReleaseBuffer(num_frames);

            packet_size = match capture_client.GetNextPacketSize() {
                Ok(s) => s,
                Err(_) => 0,
            };
        }

        std::thread::sleep(Duration::from_millis(10));
    }

    let _ = client.Stop();
    Ok(())
}
