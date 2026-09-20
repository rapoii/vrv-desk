#[cfg(windows)]
use std::mem::ManuallyDrop;
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::*;

pub struct MftH264Encoder {
    width: u32,
    height: u32,
    bitrate: u32,
    fps: f32,
    frame_seq: u16,
    sample_time_100ns: i64,
    transform: Option<IMFTransform>,
    nv12_buffer: Vec<u8>,
}

#[inline]
fn pack_u64(high: u32, low: u32) -> u64 {
    ((high as u64) << 32) | (low as u64)
}

pub fn bgra_to_nv12(width: u32, height: u32, bgra: &[u8], nv12: &mut [u8]) {
    let w = width as usize;
    let h = height as usize;
    let y_plane_size = w * h;

    for y in 0..h {
        let row_bgra_start = y * w * 4;
        let row_y_start = y * w;
        for x in 0..w {
            let bgra_idx = row_bgra_start + x * 4;
            let b = bgra[bgra_idx] as f32;
            let g = bgra[bgra_idx + 1] as f32;
            let r = bgra[bgra_idx + 2] as f32;

            let y_val = (0.299 * r + 0.587 * g + 0.114 * b).clamp(0.0, 255.0) as u8;
            nv12[row_y_start + x] = y_val;

            if (y & 1) == 0 && (x & 1) == 0 {
                let u_val = (-0.169 * r - 0.331 * g + 0.500 * b + 128.0).clamp(0.0, 255.0) as u8;
                let v_val = (0.500 * r - 0.419 * g - 0.081 * b + 128.0).clamp(0.0, 255.0) as u8;
                let uv_idx = y_plane_size + (y / 2) * w + x;
                nv12[uv_idx] = u_val;
                nv12[uv_idx + 1] = v_val;
            }
        }
    }
}

impl MftH264Encoder {
    pub fn new(
        width: u32,
        height: u32,
        bitrate: u32,
        fps: f32,
    ) -> std::result::Result<Self, String> {
        #[cfg(windows)]
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let hr_startup = MFStartup(MF_VERSION, MFSTARTUP_FULL);
            if let Err(e) = hr_startup {
                return Err(format!("MFStartup failed: {:?}", e));
            }

            let input_type_info = MFT_REGISTER_TYPE_INFO {
                guidMajorType: MFMediaType_Video,
                guidSubtype: MFVideoFormat_NV12,
            };
            let output_type_info = MFT_REGISTER_TYPE_INFO {
                guidMajorType: MFMediaType_Video,
                guidSubtype: MFVideoFormat_H264,
            };

            let flags = MFT_ENUM_FLAG(MFT_ENUM_FLAG_HARDWARE.0 | MFT_ENUM_FLAG_SORTANDFILTER.0);
            let mut activate_ptrs: *mut Option<IMFActivate> = std::ptr::null_mut();
            let mut count: u32 = 0;

            let hr = MFTEnumEx(
                MFT_CATEGORY_VIDEO_ENCODER,
                flags,
                Some(&input_type_info),
                Some(&output_type_info),
                &mut activate_ptrs,
                &mut count,
            );

            if let Err(e) = hr {
                return Err(format!("MFTEnumEx failed: {:?}", e));
            }

            if count == 0 || activate_ptrs.is_null() {
                return Err("No hardware H.264 MFT encoder found".to_string());
            }

            let activates = std::slice::from_raw_parts(activate_ptrs, count as usize);
            let mut chosen_transform: Option<IMFTransform> = None;

            for act in activates {
                if let Some(activate) = act {
                    if let Ok(transform) = activate.ActivateObject::<IMFTransform>() {
                        if let Ok(attrs) = transform.GetAttributes() {
                            let _ = attrs.SetUINT32(&MF_TRANSFORM_ASYNC_UNLOCK, 1);
                        }

                        let out_type: IMFMediaType = match MFCreateMediaType() {
                            Ok(t) => t,
                            Err(_) => continue,
                        };
                        let _ = out_type.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video);
                        let _ = out_type.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264);
                        let _ = out_type.SetUINT32(&MF_MT_AVG_BITRATE, bitrate);
                        let _ = out_type.SetUINT64(&MF_MT_FRAME_SIZE, pack_u64(width, height));
                        let _ = out_type.SetUINT64(&MF_MT_FRAME_RATE, pack_u64(fps as u32, 1));
                        let _ = out_type.SetUINT32(
                            &MF_MT_INTERLACE_MODE,
                            MFVideoInterlace_Progressive.0 as u32,
                        );

                        if transform.SetOutputType(0, &out_type, 0).is_err() {
                            continue;
                        }

                        let in_type: IMFMediaType = match MFCreateMediaType() {
                            Ok(t) => t,
                            Err(_) => continue,
                        };
                        let _ = in_type.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video);
                        let _ = in_type.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_NV12);
                        let _ = in_type.SetUINT64(&MF_MT_FRAME_SIZE, pack_u64(width, height));
                        let _ = in_type.SetUINT64(&MF_MT_FRAME_RATE, pack_u64(fps as u32, 1));
                        let _ = in_type.SetUINT32(
                            &MF_MT_INTERLACE_MODE,
                            MFVideoInterlace_Progressive.0 as u32,
                        );

                        if transform.SetInputType(0, &in_type, 0).is_err() {
                            continue;
                        }

                        let _ = transform.ProcessMessage(MFT_MESSAGE_COMMAND_FLUSH, 0);
                        let _ = transform.ProcessMessage(MFT_MESSAGE_NOTIFY_BEGIN_STREAMING, 0);
                        let _ = transform.ProcessMessage(MFT_MESSAGE_NOTIFY_START_OF_STREAM, 0);

                        chosen_transform = Some(transform);
                        break;
                    }
                }
            }

            CoTaskMemFree(Some(activate_ptrs as *const _));

            match chosen_transform {
                Some(transform) => {
                    let nv12_size = (width * height * 3 / 2) as usize;
                    Ok(Self {
                        width,
                        height,
                        bitrate,
                        fps,
                        frame_seq: 0,
                        sample_time_100ns: 0,
                        transform: Some(transform),
                        nv12_buffer: vec![0u8; nv12_size],
                    })
                }
                None => Err("Failed to configure any hardware MFT encoder".to_string()),
            }
        }

        #[cfg(not(windows))]
        {
            Err("MFT is only supported on Windows".to_string())
        }
    }

    pub fn encode_bgra(
        &mut self,
        width: u32,
        height: u32,
        bgra: &[u8],
    ) -> std::result::Result<Vec<u8>, String> {
        if self.width != width || self.height != height {
            *self = Self::new(width, height, self.bitrate, self.fps)?;
        }

        bgra_to_nv12(width, height, bgra, &mut self.nv12_buffer);

        #[cfg(windows)]
        unsafe {
            let transform = self
                .transform
                .as_ref()
                .ok_or_else(|| "Transform not initialized".to_string())?;

            let nv12_len = self.nv12_buffer.len() as u32;
            let input_media_buffer = MFCreateMemoryBuffer(nv12_len)
                .map_err(|e| format!("MFCreateMemoryBuffer failed: {:?}", e))?;

            let mut ptr: *mut u8 = std::ptr::null_mut();
            let mut max_len: u32 = 0;
            let mut cur_len: u32 = 0;
            input_media_buffer
                .Lock(&mut ptr, Some(&mut max_len), Some(&mut cur_len))
                .map_err(|e| format!("Lock input buffer failed: {:?}", e))?;

            std::ptr::copy_nonoverlapping(self.nv12_buffer.as_ptr(), ptr, self.nv12_buffer.len());
            let _ = input_media_buffer.Unlock();
            let _ = input_media_buffer.SetCurrentLength(nv12_len);

            let input_sample: IMFSample =
                MFCreateSample().map_err(|e| format!("MFCreateSample failed: {:?}", e))?;
            input_sample
                .AddBuffer(&input_media_buffer)
                .map_err(|e| format!("AddBuffer failed: {:?}", e))?;

            input_sample
                .SetSampleTime(self.sample_time_100ns)
                .map_err(|e| format!("SetSampleTime failed: {:?}", e))?;
            let frame_duration_100ns = (10_000_000.0 / self.fps) as i64;
            let _ = input_sample.SetSampleDuration(frame_duration_100ns);
            self.sample_time_100ns += frame_duration_100ns;

            // Feed input sample to MFT
            let _ = transform.ProcessInput(0, &input_sample, 0);

            // Fetch output sample
            let out_max_size = (width * height) as u32;
            let output_sample_buf = MFCreateMemoryBuffer(out_max_size)
                .map_err(|e| format!("MFCreateMemoryBuffer output failed: {:?}", e))?;

            let output_sample: IMFSample =
                MFCreateSample().map_err(|e| format!("MFCreateSample output failed: {:?}", e))?;
            let _ = output_sample.AddBuffer(&output_sample_buf);

            let mut out_buffer_struct = MFT_OUTPUT_DATA_BUFFER {
                dwStreamID: 0,
                pSample: ManuallyDrop::new(Some(output_sample)),
                dwStatus: 0,
                pEvents: ManuallyDrop::new(None),
            };

            let mut status: u32 = 0;
            let hr_out = transform.ProcessOutput(0, &mut [out_buffer_struct], &mut status);

            if let Ok(()) = hr_out {
                let mut out_ptr: *mut u8 = std::ptr::null_mut();
                let mut out_max: u32 = 0;
                let mut out_len: u32 = 0;
                if output_sample_buf
                    .Lock(&mut out_ptr, Some(&mut out_max), Some(&mut out_len))
                    .is_ok()
                {
                    let raw_nal = std::slice::from_raw_parts(out_ptr, out_len as usize).to_vec();
                    let _ = output_sample_buf.Unlock();

                    let is_idr = raw_nal.windows(4).any(|w| {
                        (w[0] == 0 && w[1] == 0 && w[2] == 0 && w[3] == 1)
                            && (w[4] & 0x1F == 5 || w[4] & 0x1F == 7)
                    });

                    let frame_type = if is_idr {
                        crate::video::FRAME_TYPE_IDR
                    } else {
                        crate::video::FRAME_TYPE_DELTA
                    };

                    let mut packet = Vec::with_capacity(8 + raw_nal.len());
                    packet.extend_from_slice(crate::video::VIDEO_MAGIC);
                    packet.push(frame_type);
                    packet.push(0x00);
                    packet.extend_from_slice(&self.frame_seq.to_be_bytes());
                    packet.extend_from_slice(&raw_nal);
                    self.frame_seq = self.frame_seq.wrapping_add(1);

                    return Ok(packet);
                }
            }

            Err("MFT did not produce output for this frame".to_string())
        }

        #[cfg(not(windows))]
        {
            Err("MFT is only supported on Windows".to_string())
        }
    }
}

unsafe impl Send for MftH264Encoder {}

impl Drop for MftH264Encoder {
    fn drop(&mut self) {
        #[cfg(windows)]
        unsafe {
            if let Some(transform) = self.transform.take() {
                let _ = transform.ProcessMessage(MFT_MESSAGE_NOTIFY_END_OF_STREAM, 0);
                drop(transform);
            }
            let _ = MFShutdown();
            CoUninitialize();
        }
    }
}
