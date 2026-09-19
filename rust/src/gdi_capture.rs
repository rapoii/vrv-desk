#[cfg(windows)]
use windows::Win32::Graphics::Gdi::*;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::*;
#[cfg(windows)]
use windows::Win32::Foundation::HWND;
use std::io::Cursor;
use image::{RgbaImage, ImageEncoder};
use image::codecs::jpeg::JpegEncoder;

#[cfg(windows)]
pub struct ScreenCapturer {
    pub screen_width: i32,
    pub screen_height: i32,
}

#[cfg(windows)]
impl ScreenCapturer {
    pub fn new() -> Result<Self, String> {
        unsafe {
            let width = GetSystemMetrics(SM_CXSCREEN);
            let height = GetSystemMetrics(SM_CYSCREEN);
            if width <= 0 || height <= 0 {
                return Err("Failed to get screen metrics".to_string());
            }
            Ok(Self {
                screen_width: width,
                screen_height: height,
            })
        }
    }

    pub fn capture_jpeg(&self, quality: u8, target_width: u32) -> Result<Vec<u8>, String> {
        unsafe {
            let hwnd = HWND(0);
            let hdc_screen = GetDC(hwnd);
            if hdc_screen.0 == 0 {
                return Err("GetDC failed".to_string());
            }

            let hdc_mem = CreateCompatibleDC(hdc_screen);
            if hdc_mem.0 == 0 {
                ReleaseDC(hwnd, hdc_screen);
                return Err("CreateCompatibleDC failed".to_string());
            }

            let w = self.screen_width;
            let h = self.screen_height;

            let hbm = CreateCompatibleBitmap(hdc_screen, w, h);
            if hbm.0 == 0 {
                DeleteDC(hdc_mem);
                ReleaseDC(hwnd, hdc_screen);
                return Err("CreateCompatibleBitmap failed".to_string());
            }

            let old_bm = SelectObject(hdc_mem, hbm);
            let blt_res = BitBlt(hdc_mem, 0, 0, w, h, hdc_screen, 0, 0, SRCCOPY);

            if let Err(e) = blt_res {
                SelectObject(hdc_mem, old_bm);
                DeleteObject(hbm);
                DeleteDC(hdc_mem);
                ReleaseDC(hwnd, hdc_screen);
                return Err(format!("BitBlt failed: {:?}", e));
            }

            let mut bi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h, // Top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [RGBQUAD::default()],
            };

            let mut raw_pixels = vec![0u8; (w * h * 4) as usize];
            let lines = GetDIBits(
                hdc_mem,
                hbm,
                0,
                h as u32,
                Some(raw_pixels.as_mut_ptr() as *mut _),
                &mut bi,
                DIB_RGB_COLORS,
            );

            // Clean up GDI handles immediately
            SelectObject(hdc_mem, old_bm);
            DeleteObject(hbm);
            DeleteDC(hdc_mem);
            ReleaseDC(hwnd, hdc_screen);

            if lines == 0 {
                return Err("GetDIBits failed".to_string());
            }

            // Convert BGRA to RGBA in-place
            for chunk in raw_pixels.chunks_exact_mut(4) {
                let b = chunk[0];
                let r = chunk[2];
                chunk[0] = r;
                chunk[2] = b;
            }

            // Calculate scaled dimensions
            let (final_w, final_h) = if (w as u32) > target_width && target_width > 0 {
                let scale = target_width as f32 / w as f32;
                (target_width, (h as f32 * scale) as u32)
            } else {
                (w as u32, h as u32)
            };

            let img = RgbaImage::from_raw(w as u32, h as u32, raw_pixels)
                .ok_or_else(|| "Failed to construct RgbaImage".to_string())?;

            let final_img = if final_w != w as u32 || final_h != h as u32 {
                image::imageops::resize(&img, final_w, final_h, image::imageops::FilterType::Triangle)
            } else {
                img
            };

            // Convert to RGB for JPEG encoding
            let rgb_img = image::DynamicImage::ImageRgba8(final_img).to_rgb8();

            let mut buffer = Cursor::new(Vec::with_capacity((final_w * final_h) as usize / 4));
            let mut encoder = JpegEncoder::new_with_quality(&mut buffer, quality);
            encoder.write_image(
                rgb_img.as_raw(),
                final_w,
                final_h,
                image::ExtendedColorType::Rgb8,
            ).map_err(|e| format!("JPEG encode error: {:?}", e))?;

            Ok(buffer.into_inner())
        }
    }
}

#[cfg(not(windows))]
pub struct ScreenCapturer;

#[cfg(not(windows))]
impl ScreenCapturer {
    pub fn new() -> Result<Self, String> {
        Ok(Self)
    }
    pub fn capture_jpeg(&self, _quality: u8, _target_width: u32) -> Result<Vec<u8>, String> {
        Err("Unsupported platform".to_string())
    }
}
