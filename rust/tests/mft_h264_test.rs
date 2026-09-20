#[cfg(windows)]
mod test {
    use windows::Win32::Media::MediaFoundation::*;
    use windows::Win32::System::Com::*;

    fn pack_u64(high: u32, low: u32) -> u64 {
        ((high as u64) << 32) | (low as u64)
    }

    fn bgra_to_nv12(width: u32, height: u32, bgra: &[u8], nv12: &mut [u8]) {
        let w = width as usize;
        let h = height as usize;
        let y_plane_size = w * h;

        // Fill Y plane and UV plane
        for y in 0..h {
            for x in 0..w {
                let bgra_idx = (y * w + x) * 4;
                let b = bgra[bgra_idx] as f32;
                let g = bgra[bgra_idx + 1] as f32;
                let r = bgra[bgra_idx + 2] as f32;

                // ITU-R BT.601
                let y_val = (0.299 * r + 0.587 * g + 0.114 * b).clamp(0.0, 255.0) as u8;
                nv12[y * w + x] = y_val;

                if y % 2 == 0 && x % 2 == 0 {
                    let u_val = (-0.169 * r - 0.331 * g + 0.500 * b + 128.0).clamp(0.0, 255.0) as u8;
                    let v_val = (0.500 * r - 0.419 * g - 0.081 * b + 128.0).clamp(0.0, 255.0) as u8;
                    let uv_idx = y_plane_size + (y / 2) * w + x;
                    nv12[uv_idx] = u_val;
                    nv12[uv_idx + 1] = v_val;
                }
            }
        }
    }

    #[test]
    fn test_bgra_to_nv12() {
        let w = 64u32;
        let h = 64u32;
        let bgra = vec![255u8; (w * h * 4) as usize];
        let mut nv12 = vec![0u8; (w * h * 3 / 2) as usize];
        bgra_to_nv12(w, h, &bgra, &mut nv12);
        assert_eq!(nv12[0], 255); // White pixel Y = 255
    }
}

#[cfg(not(windows))]
#[test]
fn test_mft() {}
