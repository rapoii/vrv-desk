#[cfg(windows)]
use windows::core::ComInterface;
#[cfg(windows)]
use windows::Win32::Foundation::RECT;
#[cfg(windows)]
use windows::Win32::Graphics::Direct3D::*;
#[cfg(windows)]
use windows::Win32::Graphics::Direct3D11::*;
#[cfg(windows)]
use windows::Win32::Graphics::Dxgi::Common::*;
#[cfg(windows)]
use windows::Win32::Graphics::Dxgi::*;

use crate::dirty_rect::{DirtyFrameInfo, DirtyRect};

use image::codecs::jpeg::JpegEncoder;
use image::{ImageEncoder, RgbaImage};
use std::io::Cursor;

#[cfg(windows)]
pub struct DxgiCapturer {
  pub screen_width: u32,
  pub screen_height: u32,
  pub width: u32,
  pub height: u32,
  pub current_monitor_index: u32,
  device: ID3D11Device,
  context: ID3D11DeviceContext,
  duplication: Option<IDXGIOutputDuplication>,
  staging_texture: ID3D11Texture2D,
}

#[cfg(windows)]
impl DxgiCapturer {
  pub fn new() -> Result<Self, String> {
    Self::new_for_monitor(0)
  }

  pub fn new_for_monitor(monitor_index: u32) -> Result<Self, String> {
    unsafe {
      let mut device: Option<ID3D11Device> = None;
      let mut context: Option<ID3D11DeviceContext> = None;
      let mut feature_level = D3D_FEATURE_LEVEL_11_0;

      let feature_levels = [
        D3D_FEATURE_LEVEL_11_1,
        D3D_FEATURE_LEVEL_11_0,
        D3D_FEATURE_LEVEL_10_1,
        D3D_FEATURE_LEVEL_10_0,
      ];

      // 1. Create Direct3D 11 Device and Context with BGRA support
      let hr = D3D11CreateDevice(
        None,
        D3D_DRIVER_TYPE_HARDWARE,
        None,
        D3D11_CREATE_DEVICE_BGRA_SUPPORT,
        Some(&feature_levels),
        D3D11_SDK_VERSION,
        Some(&mut device),
        Some(&mut feature_level),
        Some(&mut context),
      );

      if hr.is_err() || device.is_none() || context.is_none() {
        return Err(format!("D3D11CreateDevice failed: {:?}", hr));
      }

      let device = device.unwrap();
      let context = context.unwrap();

      // 2. Query IDXGIDevice -> IDXGIAdapter -> IDXGIOutput -> IDXGIOutput1 -> DuplicateOutput
      let dxgi_device: IDXGIDevice = device
        .cast()
        .map_err(|e| format!("Failed to cast ID3D11Device to IDXGIDevice: {:?}", e))?;

      let adapter = dxgi_device
        .GetAdapter()
        .map_err(|e| format!("GetAdapter failed: {:?}", e))?;

      let output = adapter
        .EnumOutputs(monitor_index)
        .map_err(|e| format!("EnumOutputs({}) failed: {:?}", monitor_index, e))?;

      let output1: IDXGIOutput1 = output
        .cast()
        .map_err(|e| format!("Failed to cast IDXGIOutput to IDXGIOutput1: {:?}", e))?;

      let mut output_desc = DXGI_OUTPUT_DESC::default();
      output
        .GetDesc(&mut output_desc)
        .map_err(|e| format!("GetDesc failed: {:?}", e))?;

      let width = (output_desc.DesktopCoordinates.right - output_desc.DesktopCoordinates.left)
        .abs() as u32;
      let height = (output_desc.DesktopCoordinates.bottom
        - output_desc.DesktopCoordinates.top)
        .abs() as u32;

      if width == 0 || height == 0 {
        return Err(format!(
          "Invalid desktop dimensions for monitor {}: {}x{}",
          monitor_index, width, height
        ));
      }

      let duplication = output1
        .DuplicateOutput(&device)
        .map_err(|e| format!("DuplicateOutput failed: {:?}", e))?;

      // 3. Create Staging Texture in CPU accessible memory
      let staging_desc = D3D11_TEXTURE2D_DESC {
        Width: width,
        Height: height,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
          Count: 1,
          Quality: 0,
        },
        Usage: D3D11_USAGE_STAGING,
        BindFlags: 0,
        CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
        MiscFlags: 0,
      };

      let mut staging_texture: Option<ID3D11Texture2D> = None;
      device
        .CreateTexture2D(&staging_desc, None, Some(&mut staging_texture))
        .map_err(|e| format!("CreateTexture2D for staging texture failed: {:?}", e))?;

      let staging_texture = staging_texture
        .ok_or_else(|| "Staging texture was None after creation".to_string())?;

      Ok(Self {
        screen_width: width,
        screen_height: height,
        width,
        height,
        current_monitor_index: monitor_index,
        device,
        context,
        duplication: Some(duplication),
        staging_texture,
      })
    }
  }

  pub fn switch_monitor(&mut self, monitor_index: u32) -> Result<(u32, u32), String> {
    if self.current_monitor_index == monitor_index {
      return Ok((self.width, self.height));
    }

    unsafe {
      // Drop old duplication first to release DXGI resource
      self.duplication = None;

      let dxgi_device: IDXGIDevice = self
        .device
        .cast()
        .map_err(|e| format!("Failed to cast ID3D11Device to IDXGIDevice: {:?}", e))?;

      let adapter = dxgi_device
        .GetAdapter()
        .map_err(|e| format!("GetAdapter failed: {:?}", e))?;

      let output = adapter
        .EnumOutputs(monitor_index)
        .map_err(|e| format!("EnumOutputs({}) failed: {:?}", monitor_index, e))?;

      let output1: IDXGIOutput1 = output
        .cast()
        .map_err(|e| format!("Failed to cast IDXGIOutput to IDXGIOutput1: {:?}", e))?;

      let mut output_desc = DXGI_OUTPUT_DESC::default();
      output
        .GetDesc(&mut output_desc)
        .map_err(|e| format!("GetDesc failed: {:?}", e))?;

      let width = (output_desc.DesktopCoordinates.right - output_desc.DesktopCoordinates.left)
        .abs() as u32;
      let height = (output_desc.DesktopCoordinates.bottom
        - output_desc.DesktopCoordinates.top)
        .abs() as u32;

      if width == 0 || height == 0 {
        return Err(format!(
          "Invalid desktop dimensions for monitor {}: {}x{}",
          monitor_index, width, height
        ));
      }

      let duplication = output1.DuplicateOutput(&self.device).map_err(|e| {
        format!(
          "DuplicateOutput for monitor {} failed: {:?}",
          monitor_index, e
        )
      })?;

      let staging_desc = D3D11_TEXTURE2D_DESC {
        Width: width,
        Height: height,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
          Count: 1,
          Quality: 0,
        },
        Usage: D3D11_USAGE_STAGING,
        BindFlags: 0,
        CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
        MiscFlags: 0,
      };

      let mut staging_texture: Option<ID3D11Texture2D> = None;
      self.device
        .CreateTexture2D(&staging_desc, None, Some(&mut staging_texture))
        .map_err(|e| format!("CreateTexture2D for staging texture failed: {:?}", e))?;

      let staging_texture = staging_texture
        .ok_or_else(|| "Staging texture was None after creation".to_string())?;

      self.screen_width = width;
      self.screen_height = height;
      self.width = width;
      self.height = height;
      self.current_monitor_index = monitor_index;
      self.duplication = Some(duplication);
      self.staging_texture = staging_texture;

      Ok((width, height))
    }
  }

  /// Reinitialize the DXGI output duplication (e.g. after DXGI_ERROR_ACCESS_LOST)
  pub fn reinitialize(&mut self) -> Result<(), String> {
    unsafe {
      self.duplication = None;

      let dxgi_device: IDXGIDevice = self
        .device
        .cast()
        .map_err(|e| format!("Failed to cast ID3D11Device to IDXGIDevice: {:?}", e))?;

      let adapter = dxgi_device
        .GetAdapter()
        .map_err(|e| format!("GetAdapter failed: {:?}", e))?;

      let output = adapter
        .EnumOutputs(self.current_monitor_index)
        .map_err(|e| {
          format!(
            "EnumOutputs({}) failed: {:?}",
            self.current_monitor_index, e
          )
        })?;

      let output1: IDXGIOutput1 = output
        .cast()
        .map_err(|e| format!("Failed to cast IDXGIOutput to IDXGIOutput1: {:?}", e))?;

      let duplication = output1
        .DuplicateOutput(&self.device)
        .map_err(|e| format!("DuplicateOutput re-init failed: {:?}", e))?;

      self.duplication = Some(duplication);
      Ok(())
    }
  }

  pub fn acquire_next_frame(&mut self, timeout_ms: u32) -> Result<Option<Vec<u8>>, String> {
    self.capture_jpeg(timeout_ms, 75, self.screen_width)
  }

  /// Acquire the next frame from GPU, map to CPU, convert BGRA -> RGBA/JPEG.
  /// Returns Ok(None) if timeout elapsed without screen update (DXGI_ERROR_WAIT_TIMEOUT).
  pub fn capture_jpeg(
    &mut self,
    timeout_ms: u32,
    quality: u8,
    target_width: u32,
  ) -> Result<Option<Vec<u8>>, String> {
    unsafe {
      let duplication = match self.duplication.as_ref() {
        Some(d) => d,
        None => {
          self.reinitialize()?;
          match self.duplication.as_ref() {
            Some(d) => d,
            None => return Err("Duplication unavailable".to_string()),
          }
        }
      };

      let mut frame_info = DXGI_OUTDUPL_FRAME_INFO::default();
      let mut desktop_resource: Option<IDXGIResource> = None;

      let hr =
        duplication.AcquireNextFrame(timeout_ms, &mut frame_info, &mut desktop_resource);

      if let Err(e) = hr {
        if e.code() == DXGI_ERROR_WAIT_TIMEOUT {
          return Ok(None);
        }
        if e.code() == DXGI_ERROR_ACCESS_LOST {
          eprintln!(" DXGI Output Duplication access lost, reinitializing...");
          self.reinitialize()?;
          return Ok(None);
        }
        return Err(format!("AcquireNextFrame failed: {:?}", e));
      }

      let desktop_resource = match desktop_resource {
        Some(r) => r,
        None => {
          let _ = duplication.ReleaseFrame();
          return Ok(None);
        }
      };

      let gpu_texture: ID3D11Texture2D = match desktop_resource.cast() {
        Ok(t) => t,
        Err(e) => {
          let _ = duplication.ReleaseFrame();
          return Err(format!(
            "Failed to cast IDXGIResource to ID3D11Texture2D: {:?}",
            e
          ));
        }
      };

      // Copy GPU desktop texture to CPU-readable staging texture
      self.context
        .CopyResource(&self.staging_texture, &gpu_texture);

      // Map staging texture for CPU read
      let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
      let map_res = self.context.Map(
        &self.staging_texture,
        0,
        D3D11_MAP_READ,
        0,
        Some(&mut mapped),
      );

      if let Err(e) = map_res {
        let _ = duplication.ReleaseFrame();
        return Err(format!("Map staging texture failed: {:?}", e));
      }

      let width = self.screen_width;
      let height = self.screen_height;
      let row_pitch = mapped.RowPitch as usize;
      let src_slice =
        std::slice::from_raw_parts(mapped.pData as *const u8, row_pitch * height as usize);

      // Convert BGRA to RGBA row by row
      let mut raw_pixels = vec![0u8; (width * height * 4) as usize];
      for y in 0..height as usize {
        let src_row = &src_slice[y * row_pitch..(y * row_pitch + (width as usize * 4))];
        let dst_row =
          &mut raw_pixels[y * (width as usize * 4)..(y + 1) * (width as usize * 4)];
        for (src_px, dst_px) in src_row.chunks_exact(4).zip(dst_row.chunks_exact_mut(4)) {
          // src is BGRA -> dst is RGBA
          dst_px[0] = src_px[2]; // R
          dst_px[1] = src_px[1]; // G
          dst_px[2] = src_px[0]; // B
          dst_px[3] = src_px[3]; // A
        }
      }

      // Unmap staging texture and release frame immediately to unblock GPU
      self.context.Unmap(&self.staging_texture, 0);
      let _ = duplication.ReleaseFrame();

      // Calculate scaled dimensions if target_width is requested and smaller than source
      let (final_w, final_h) = if width > target_width && target_width > 0 {
        let scale = target_width as f32 / width as f32;
        (target_width, (height as f32 * scale) as u32)
      } else {
        (width, height)
      };

      let img = RgbaImage::from_raw(width, height, raw_pixels)
        .ok_or_else(|| "Failed to construct RgbaImage from DXGI buffer".to_string())?;

      let final_img = if final_w != width || final_h != height {
        image::imageops::resize(
          &img,
          final_w,
          final_h,
          image::imageops::FilterType::Triangle,
        )
      } else {
        img
      };

      let rgb_img = image::DynamicImage::ImageRgba8(final_img).to_rgb8();
      let mut buffer = Cursor::new(Vec::with_capacity((final_w * final_h) as usize / 4));
      let encoder = JpegEncoder::new_with_quality(&mut buffer, quality);
      encoder
        .write_image(
          rgb_img.as_raw(),
          final_w,
          final_h,
          image::ExtendedColorType::Rgb8,
        )
        .map_err(|e| format!("JPEG encode error: {:?}", e))?;

      Ok(Some(buffer.into_inner()))
    }
  }

  /// Acquire the next frame from GPU and return raw BGRA buffer with dirty region metadata.
  /// Returns Ok(None) if timeout elapsed or no pixels changed (bypassing staging copy).
  pub fn capture_raw_bgra_with_dirty(
    &mut self,
    timeout_ms: u32,
  ) -> Result<Option<(u32, u32, Vec<u8>, DirtyFrameInfo)>, String> {
    unsafe {
      let duplication = match self.duplication.as_ref() {
        Some(d) => d,
        None => {
          self.reinitialize()?;
          match self.duplication.as_ref() {
            Some(d) => d,
            None => return Err("Duplication unavailable".to_string()),
          }
        }
      };

      let mut frame_info = DXGI_OUTDUPL_FRAME_INFO::default();
      let mut desktop_resource: Option<IDXGIResource> = None;

      let hr =
        duplication.AcquireNextFrame(timeout_ms, &mut frame_info, &mut desktop_resource);

      if let Err(e) = hr {
        if e.code() == DXGI_ERROR_WAIT_TIMEOUT {
          return Ok(None);
        }
        if e.code() == DXGI_ERROR_ACCESS_LOST {
          eprintln!(" DXGI Output Duplication access lost, reinitializing...");
          self.reinitialize()?;
          return Ok(None);
        }
        return Err(format!("AcquireNextFrame failed: {:?}", e));
      }

      let width = self.screen_width;
      let height = self.screen_height;

      // Extract dirty rects from DXGI metadata
      let mut dirty_rects = Vec::new();
      if frame_info.TotalMetadataBufferSize > 0 {
        let mut buffer_size = 0u32;
        let _ = duplication.GetFrameDirtyRects(0, std::ptr::null_mut(), &mut buffer_size);
        if buffer_size > 0 {
          let count = (buffer_size as usize) / std::mem::size_of::<RECT>();
          let mut rect_buf = vec![RECT::default(); count];
          let mut actual_size = 0u32;
          if duplication
            .GetFrameDirtyRects(buffer_size, rect_buf.as_mut_ptr(), &mut actual_size)
            .is_ok()
          {
            for r in rect_buf {
              let dr = DirtyRect::new(r.left, r.top, r.right, r.bottom);
              if !dr.is_empty() {
                dirty_rects.push(dr);
              }
            }
          }
        }
      }

      if dirty_rects.is_empty() && frame_info.AccumulatedFrames > 0 {
        // Whole frame changed / coalesced
        dirty_rects.push(DirtyRect::new(0, 0, width as i32, height as i32));
      }

      // Zero-overhead check: If no frames accumulated and no dirty rects found, bypass!
      if frame_info.AccumulatedFrames == 0 && dirty_rects.is_empty() {
        let _ = duplication.ReleaseFrame();
        return Ok(None);
      }

      let desktop_resource = match desktop_resource {
        Some(r) => r,
        None => {
          let _ = duplication.ReleaseFrame();
          return Ok(None);
        }
      };

      let gpu_texture: ID3D11Texture2D = match desktop_resource.cast() {
        Ok(t) => t,
        Err(e) => {
          let _ = duplication.ReleaseFrame();
          return Err(format!(
            "Failed to cast IDXGIResource to ID3D11Texture2D: {:?}",
            e
          ));
        }
      };

      // Copy GPU desktop texture to CPU-readable staging texture
      self.context
        .CopyResource(&self.staging_texture, &gpu_texture);

      // Map staging texture for CPU read
      let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
      let map_res = self.context.Map(
        &self.staging_texture,
        0,
        D3D11_MAP_READ,
        0,
        Some(&mut mapped),
      );

      if let Err(e) = map_res {
        let _ = duplication.ReleaseFrame();
        return Err(format!("Map staging texture failed: {:?}", e));
      }

      let row_pitch = mapped.RowPitch as usize;
      let src_slice =
        std::slice::from_raw_parts(mapped.pData as *const u8, row_pitch * height as usize);

      let mut raw_pixels = vec![0u8; (width * height * 4) as usize];
      for y in 0..height as usize {
        let src_row = &src_slice[y * row_pitch..(y * row_pitch + (width as usize * 4))];
        let dst_row =
          &mut raw_pixels[y * (width as usize * 4)..(y + 1) * (width as usize * 4)];
        dst_row.copy_from_slice(src_row);
      }

      self.context.Unmap(&self.staging_texture, 0);
      let _ = duplication.ReleaseFrame();

      let dirty_info = DirtyFrameInfo::new(width, height, dirty_rects);
      Ok(Some((width, height, raw_pixels, dirty_info)))
    }
  }

  /// Acquire the next frame from GPU and return raw BGRA buffer directly.
  /// Returns Ok(None) if timeout elapsed without screen update (DXGI_ERROR_WAIT_TIMEOUT).
  pub fn capture_raw_bgra(
    &mut self,
    timeout_ms: u32,
  ) -> Result<Option<(u32, u32, Vec<u8>)>, String> {
    self.capture_raw_bgra_with_dirty(timeout_ms)
      .map(|opt| opt.map(|(w, h, bgra, _dirty)| (w, h, bgra)))
  }
}

#[cfg(not(windows))]
pub struct DxgiCapturer {
  pub screen_width: u32,
  pub screen_height: u32,
  pub width: u32,
  pub height: u32,
  pub current_monitor_index: u32,
}

#[cfg(not(windows))]
impl DxgiCapturer {
  pub fn new() -> Result<Self, String> {
    Self::new_for_monitor(0)
  }

  pub fn new_for_monitor(_monitor_index: u32) -> Result<Self, String> {
    Err("DXGI is only supported on Windows".to_string())
  }

  pub fn switch_monitor(&mut self, _monitor_index: u32) -> Result<(u32, u32), String> {
    Err("DXGI is only supported on Windows".to_string())
  }

  pub fn acquire_next_frame(&mut self, _timeout_ms: u32) -> Result<Option<Vec<u8>>, String> {
    Err("DXGI is only supported on Windows".to_string())
  }

  pub fn capture_jpeg(
    &mut self,
    _timeout_ms: u32,
    _quality: u8,
    _target_width: u32,
  ) -> Result<Option<Vec<u8>>, String> {
    Err("DXGI is only supported on Windows".to_string())
  }

  pub fn capture_raw_bgra_with_dirty(
    &mut self,
    _timeout_ms: u32,
  ) -> Result<Option<(u32, u32, Vec<u8>, DirtyFrameInfo)>, String> {
    Err("DXGI is only supported on Windows".to_string())
  }
}

/// Resilient screen capturer combining hardware DXGI Desktop Duplication with automatic GDI fallback and Headless Virtual Canvas
pub struct HybridScreenCapturer {
  dxgi: Option<DxgiCapturer>,
  gdi: Option<crate::gdi_capture::ScreenCapturer>,
  is_dxgi: bool,
  pub is_headless: bool,
  headless_frame_count: u64,
  width: u32,
  height: u32,
}

impl HybridScreenCapturer {
  pub fn new() -> Result<Self, String> {
    #[cfg(windows)]
    {
      match DxgiCapturer::new() {
        Ok(dxgi) => {
          let w = dxgi.screen_width;
          let h = dxgi.screen_height;
          println!(
            " Initialized DirectX 11 DXGI GPU capture engine ({}x{})",
            w, h
          );
          Ok(Self {
            dxgi: Some(dxgi),
            gdi: None,
            is_dxgi: true,
            is_headless: false,
            headless_frame_count: 0,
            width: w,
            height: h,
          })
        }
        Err(e) => {
          println!(" DXGI unavailable ({}), falling back to GDI BitBlt", e);
          match crate::gdi_capture::ScreenCapturer::new() {
            Ok(gdi) => {
              let w = gdi.screen_width as u32;
              let h = gdi.screen_height as u32;
              println!(" Initialized GDI BitBlt capture engine ({}x{})", w, h);
              Ok(Self {
                dxgi: None,
                gdi: Some(gdi),
                is_dxgi: false,
                is_headless: false,
                headless_frame_count: 0,
                width: w,
                height: h,
              })
            }
            Err(ge) => {
              println!(" GDI unavailable ({}) - initializing Headless Virtual Canvas (1920x1080)", ge);
              Ok(Self {
                dxgi: None,
                gdi: None,
                is_dxgi: false,
                is_headless: true,
                headless_frame_count: 0,
                width: 1920,
                height: 1080,
              })
            }
          }
        }
      }
    }

    #[cfg(not(windows))]
    {
      Err("Unsupported platform for screen capture".to_string())
    }
  }

  pub fn screen_width(&self) -> u32 {
    self.width
  }

  pub fn screen_height(&self) -> u32 {
    self.height
  }

  pub fn switch_monitor(&mut self, monitor_index: u32) -> Result<(u32, u32), String> {
    if let Some(ref mut dxgi) = self.dxgi {
      let (w, h) = dxgi.switch_monitor(monitor_index)?;
      self.width = w;
      self.height = h;
      Ok((w, h))
    } else {
      Ok((self.width, self.height))
    }
  }

  pub fn is_dxgi(&self) -> bool {
    self.is_dxgi
  }

  pub fn capture_jpeg(
    &mut self,
    timeout_ms: u32,
    quality: u8,
    target_width: u32,
  ) -> Result<Option<Vec<u8>>, String> {
    if self.is_headless {
      let bgra = crate::virtual_display::generate_headless_frame(
        self.width,
        self.height,
        self.headless_frame_count,
        "Headless Canvas",
      );
      self.headless_frame_count += 1;
      let mut raw_pixels = vec![0u8; (self.width * self.height * 4) as usize];
      for (src_px, dst_px) in bgra.chunks_exact(4).zip(raw_pixels.chunks_exact_mut(4)) {
        dst_px[0] = src_px[2]; // R
        dst_px[1] = src_px[1]; // G
        dst_px[2] = src_px[0]; // B
        dst_px[3] = src_px[3]; // A
      }
      let img = image::RgbaImage::from_raw(self.width, self.height, raw_pixels)
        .ok_or_else(|| "Failed to construct RgbaImage from headless buffer".to_string())?;
      let rgb_img = image::DynamicImage::ImageRgba8(img).to_rgb8();
      let mut buffer =
        std::io::Cursor::new(Vec::with_capacity((self.width * self.height) as usize / 4));
      let mut encoder =
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, quality);
      encoder
        .encode_image(&rgb_img)
        .map_err(|e| format!("Headless JPEG encode failed: {:?}", e))?;
      return Ok(Some(buffer.into_inner()));
    }

    if self.is_dxgi {
      if let Some(ref mut dxgi) = self.dxgi {
        match dxgi.capture_jpeg(timeout_ms, quality, target_width) {
          Ok(res) => return Ok(res),
          Err(e) => {
            eprintln!(" DXGI capture error: {}, falling back to GDI engine", e);
            self.is_dxgi = false;
            self.dxgi = None;
            if let Ok(gdi) = crate::gdi_capture::ScreenCapturer::new() {
              self.gdi = Some(gdi);
            } else {
              self.is_headless = true;
            }
          }
        }
      }
    }

    // Fallback or primary GDI execution
    if let Some(ref gdi) = self.gdi {
      match gdi.capture_jpeg(quality, target_width) {
        Ok(jpeg) => return Ok(Some(jpeg)),
        Err(e) => {
          eprintln!(" GDI capture error: {}, switching to headless mode", e);
          self.is_headless = true;
          self.gdi = None;
        }
      }
    }

    if self.is_headless {
      let bgra = crate::virtual_display::generate_headless_frame(
        self.width,
        self.height,
        self.headless_frame_count,
        "Headless Canvas",
      );
      self.headless_frame_count += 1;
      let mut raw_pixels = vec![0u8; (self.width * self.height * 4) as usize];
      for (src_px, dst_px) in bgra.chunks_exact(4).zip(raw_pixels.chunks_exact_mut(4)) {
        dst_px[0] = src_px[2];
        dst_px[1] = src_px[1];
        dst_px[2] = src_px[0];
        dst_px[3] = src_px[3];
      }
      let img = image::RgbaImage::from_raw(self.width, self.height, raw_pixels)
        .ok_or_else(|| "Failed to construct RgbaImage from headless buffer".to_string())?;
      let rgb_img = image::DynamicImage::ImageRgba8(img).to_rgb8();
      let mut buffer =
        std::io::Cursor::new(Vec::with_capacity((self.width * self.height) as usize / 4));
      let mut encoder =
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, quality);
      encoder
        .encode_image(&rgb_img)
        .map_err(|e| format!("Headless JPEG encode failed: {:?}", e))?;
      return Ok(Some(buffer.into_inner()));
    }

    let gdi = crate::gdi_capture::ScreenCapturer::new()?;
    let jpeg = gdi.capture_jpeg(quality, target_width)?;
    self.gdi = Some(gdi);
    Ok(Some(jpeg))
  }

  /// Capture raw BGRA buffer along with dirty frame metrics.
  pub fn capture_raw_bgra_with_dirty(
    &mut self,
    timeout_ms: u32,
  ) -> Result<Option<(u32, u32, Vec<u8>, DirtyFrameInfo)>, String> {
    #[cfg(windows)]
    {
      if self.is_headless {
        let bgra = crate::virtual_display::generate_headless_frame(
          self.width,
          self.height,
          self.headless_frame_count,
          "Headless Canvas",
        );
        self.headless_frame_count += 1;
        let dirty = DirtyFrameInfo::new(
          self.width,
          self.height,
          vec![DirtyRect::new(0, 0, self.width as i32, self.height as i32)],
        );
        return Ok(Some((self.width, self.height, bgra, dirty)));
      }

      if self.is_dxgi {
        if let Some(ref mut dxgi) = self.dxgi {
          match dxgi.capture_raw_bgra_with_dirty(timeout_ms) {
            Ok(res) => return Ok(res),
            Err(e) => {
              eprintln!(" DXGI capture error: {}, falling back to GDI engine", e);
              self.is_dxgi = false;
              self.dxgi = None;
              if let Ok(gdi) = crate::gdi_capture::ScreenCapturer::new() {
                self.gdi = Some(gdi);
              } else {
                self.is_headless = true;
              }
            }
          }
        }
      }

      if let Some(ref gdi) = self.gdi {
        match gdi.capture_raw_bgra() {
          Ok((w, h, bgra)) => {
            let dirty = DirtyFrameInfo::new(
              w,
              h,
              vec![DirtyRect::new(0, 0, w as i32, h as i32)],
            );
            return Ok(Some((w, h, bgra, dirty)));
          }
          Err(e) => {
            eprintln!(" GDI capture error: {}, switching to headless mode", e);
            self.is_headless = true;
            self.gdi = None;
          }
        }
      }

      if self.is_headless {
        let bgra = crate::virtual_display::generate_headless_frame(
          self.width,
          self.height,
          self.headless_frame_count,
          "Headless Canvas",
        );
        self.headless_frame_count += 1;
        let dirty = DirtyFrameInfo::new(
          self.width,
          self.height,
          vec![DirtyRect::new(0, 0, self.width as i32, self.height as i32)],
        );
        return Ok(Some((self.width, self.height, bgra, dirty)));
      }

      let gdi = crate::gdi_capture::ScreenCapturer::new()?;
      let (w, h, bgra) = gdi.capture_raw_bgra()?;
      let dirty = DirtyFrameInfo::new(w, h, vec![DirtyRect::new(0, 0, w as i32, h as i32)]);
      self.gdi = Some(gdi);
      Ok(Some((w, h, bgra, dirty)))
    }

    #[cfg(not(windows))]
    {
      let _ = timeout_ms;
      Err("Screen capture only supported on Windows".to_string())
    }
  }

  /// Capture frame from GPU/GDI and encode with H.264 real-time compression with dirty region metadata
  pub fn capture_h264_with_dirty(
    &mut self,
    timeout_ms: u32,
    encoder: &mut crate::video::VideoEncoder,
  ) -> Result<Option<(Vec<u8>, DirtyFrameInfo)>, String> {
    match self.capture_raw_bgra_with_dirty(timeout_ms)? {
      Some((w, h, bgra, dirty)) => {
        let packet = encoder.encode_bgra(w, h, &bgra)?;
        Ok(Some((packet, dirty)))
      }
      None => Ok(None),
    }
  }

  /// Capture frame from GPU/GDI and encode with H.264 real-time compression (VH24 packet)
  pub fn capture_h264(
    &mut self,
    timeout_ms: u32,
    encoder: &mut crate::video::VideoEncoder,
  ) -> Result<Option<Vec<u8>>, String> {
    self.capture_h264_with_dirty(timeout_ms, encoder)
      .map(|opt| opt.map(|(pkt, _dirty)| pkt))
  }

  /// Capture frame from GPU/GDI, optionally downscale, and encode with H.264
  pub fn capture_h264_scaled_with_dirty(
    &mut self,
    timeout_ms: u32,
    encoder: &mut crate::video::VideoEncoder,
    target_w: u32,
    target_h: u32,
    scale_buf: &mut Vec<u8>,
  ) -> Result<Option<(Vec<u8>, DirtyFrameInfo)>, String> {
    match self.capture_raw_bgra_with_dirty(timeout_ms)? {
      Some((w, h, bgra, dirty)) => {
        let packet = if target_w != w || target_h != h {
          crate::video::scale_bgra(&bgra, w, h, target_w, target_h, scale_buf);
          encoder.encode_bgra(target_w, target_h, scale_buf)?
        } else {
          encoder.encode_bgra(w, h, &bgra)?
        };
        Ok(Some((packet, dirty)))
      }
      None => Ok(None),
    }
  }
}
