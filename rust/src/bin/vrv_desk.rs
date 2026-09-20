#![windows_subsystem = "windows"]

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Dwm::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

use mirror_core::host_service::{
    format_six_digit_display, get_host_name, get_or_generate_device_id, get_or_generate_pin,
    run_host_server_loop, HostTelemetry,
};
use mirror_core::platform::windows_input::set_clipboard_text;

// Colors
const COLOR_BG: u32 = 0x0017120F; // #0F1217 in 0x00BBGGRR
const COLOR_CARD: u32 = 0x002B211A; // #1A212B
const COLOR_CARD_BORDER: u32 = 0x00453528; // #283545
const COLOR_ACCENT_CYAN: u32 = 0x00E2B838; // #38B8E2
const COLOR_ACCENT_GREEN: u32 = 0x006ED710; // #10D76E
const COLOR_ACCENT_RED: u32 = 0x004545EF; // #EF4545
const COLOR_TEXT_WHITE: u32 = 0x00F8FAFC;
const COLOR_TEXT_MUTED: u32 = 0x009CA3AF;
const COLOR_CONSOLE_BG: u32 = 0x00120E0A;

struct AppState {
    device_id: String,
    pin: Arc<std::sync::RwLock<String>>,
    unattended: Arc<std::sync::RwLock<mirror_core::unattended::UnattendedConfig>>,
    host_name: String,
    telemetry: HostTelemetry,
    is_sharing: Arc<AtomicBool>,
    hover_button: Option<u32>,
    hwnd: HWND,
    edit_remote_id: HWND,
    edit_remote_pin: HWND,
}

static mut APP_STATE: Option<AppState> = None;

// Button IDs for hit testing
const BTN_COPY_ID: u32 = 1;
const BTN_NEW_PIN: u32 = 2;
const BTN_TOGGLE_HOST: u32 = 3;
const BTN_CONNECT: u32 = 4;
const BTN_CLEAR_LOG: u32 = 5;
const BTN_UNATTENDED: u32 = 6;

// Button Rectangles
const RECT_BTN_COPY_ID: RECT = RECT { left: 420, top: 135, right: 510, bottom: 175 };
const RECT_BTN_NEW_PIN: RECT = RECT { left: 420, top: 195, right: 510, bottom: 235 };
const RECT_BTN_TOGGLE_HOST: RECT = RECT { left: 45, top: 255, right: 230, bottom: 295 };
const RECT_BTN_UNATTENDED: RECT = RECT { left: 245, top: 255, right: 510, bottom: 295 };
const RECT_BTN_CONNECT: RECT = RECT { left: 565, top: 255, right: 835, bottom: 295 };
const RECT_BTN_CLEAR_LOG: RECT = RECT { left: 745, top: 405, right: 835, bottom: 430 };

fn point_in_rect(pt: POINT, r: RECT) -> bool {
    pt.x >= r.left && pt.x <= r.right && pt.y >= r.top && pt.y <= r.bottom
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            // Enable Windows 11 Dark Mode Title Bar
            let dark_mode: BOOL = BOOL(1);
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWINDOWATTRIBUTE(20), // DWMWA_USE_IMMERSIVE_DARK_MODE
                &dark_mode as *const _ as *const c_void,
                std::mem::size_of::<BOOL>() as u32,
            );

            // Create Timer for smooth 2Hz GUI updates
            SetTimer(hwnd, 1001, 500, None);

            // Create Edit Controls for Remote Connection
            let hinstance = GetModuleHandleW(None).unwrap();
            let edit_id = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("EDIT"),
                w!(""),
                WS_CHILD | WS_VISIBLE | WS_BORDER | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
                565,
                150,
                270,
                28,
                hwnd,
                HMENU(101 as _),
                hinstance,
                None,
            );

            let edit_pin = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("EDIT"),
                w!(""),
                WS_CHILD | WS_VISIBLE | WS_BORDER | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
                565,
                210,
                270,
                28,
                hwnd,
                HMENU(102 as _),
                hinstance,
                None,
            );

            if let Some(ref mut state) = APP_STATE {
                state.hwnd = hwnd;
                state.edit_remote_id = edit_id;
                state.edit_remote_pin = edit_pin;
            }

            LRESULT(0)
        }

        WM_TIMER => {
            if wparam.0 == 1001 {
                InvalidateRect(hwnd, None, BOOL(0));
            }
            LRESULT(0)
        }

        WM_MOUSEMOVE => {
            let x = (lparam.0 & 0xffff) as i32;
            let y = ((lparam.0 >> 16) & 0xffff) as i32;
            let pt = POINT { x, y };

            let mut new_hover = None;
            if point_in_rect(pt, RECT_BTN_COPY_ID) {
                new_hover = Some(BTN_COPY_ID);
            } else if point_in_rect(pt, RECT_BTN_NEW_PIN) {
                new_hover = Some(BTN_NEW_PIN);
            } else if point_in_rect(pt, RECT_BTN_TOGGLE_HOST) {
                new_hover = Some(BTN_TOGGLE_HOST);
            } else if point_in_rect(pt, RECT_BTN_UNATTENDED) {
                new_hover = Some(BTN_UNATTENDED);
            } else if point_in_rect(pt, RECT_BTN_CONNECT) {
                new_hover = Some(BTN_CONNECT);
            } else if point_in_rect(pt, RECT_BTN_CLEAR_LOG) {
                new_hover = Some(BTN_CLEAR_LOG);
            }

            if let Some(ref mut state) = APP_STATE {
                if state.hover_button != new_hover {
                    state.hover_button = new_hover;
                    InvalidateRect(hwnd, None, BOOL(0));
                }
            }

            LRESULT(0)
        }

        WM_LBUTTONDOWN => {
            let x = (lparam.0 & 0xffff) as i32;
            let y = ((lparam.0 >> 16) & 0xffff) as i32;
            let pt = POINT { x, y };

            if let Some(ref mut state) = APP_STATE {
                if point_in_rect(pt, RECT_BTN_COPY_ID) {
                    let id_clean = state.device_id.replace(' ', "");
                    set_clipboard_text(&id_clean);
                    state.telemetry.add_log(format!("Device ID ({}) copied to clipboard!", id_clean));
                    InvalidateRect(hwnd, None, BOOL(0));
                } else if point_in_rect(pt, RECT_BTN_NEW_PIN) {
                    let new_pin = get_or_generate_pin(None);
                    *state.pin.write().unwrap() = new_pin.clone();
                    state.telemetry.add_log(format!("New PIN generated: {}", format_six_digit_display(&new_pin)));
                    InvalidateRect(hwnd, None, BOOL(0));
                } else if point_in_rect(pt, RECT_BTN_TOGGLE_HOST) {
                    let current = state.is_sharing.load(Ordering::Relaxed);
                    state.is_sharing.store(!current, Ordering::Relaxed);
                    let status = if !current { "resumed" } else { "paused" };
                    state.telemetry.add_log(format!("Screen Sharing is now {}", status));
                    InvalidateRect(hwnd, None, BOOL(0));
                } else if point_in_rect(pt, RECT_BTN_UNATTENDED) {
                    let mut cfg = state.unattended.write().unwrap();
                    let new_state = !cfg.enabled;
                    cfg.enabled = new_state;
                    if new_state && cfg.password_hash.is_none() {
                        cfg.set_password("vrv2026");
                    }
                    let _ = cfg.save();
                    let status = if new_state { "ENABLED (Pass: vrv2026)" } else { "DISABLED" };
                    state.telemetry.add_log(format!("Unattended Access is now {}", status));
                    InvalidateRect(hwnd, None, BOOL(0));
                } else if point_in_rect(pt, RECT_BTN_CLEAR_LOG) {
                    if let Ok(mut logs) = state.telemetry.log_messages.write() {
                        logs.clear();
                    }
                    InvalidateRect(hwnd, None, BOOL(0));
                } else if point_in_rect(pt, RECT_BTN_CONNECT) {
                    state.telemetry.add_log("Connecting to remote device...".to_string());
                    InvalidateRect(hwnd, None, BOOL(0));
                }
            }

            LRESULT(0)
        }

        WM_CTLCOLOREDIT => {
            let hdc = HDC(wparam.0 as _);
            SetTextColor(hdc, COLORREF(COLOR_TEXT_WHITE));
            SetBkColor(hdc, COLORREF(COLOR_CONSOLE_BG));
            let brush = CreateSolidBrush(COLORREF(COLOR_CONSOLE_BG));
            LRESULT(brush.0 as _)
        }

        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);

            // Double buffering to eliminate all flicker
            let mut client_rect = RECT::default();
            GetClientRect(hwnd, &mut client_rect);
            let width = client_rect.right - client_rect.left;
            let height = client_rect.bottom - client_rect.top;

            let mem_dc = CreateCompatibleDC(hdc);
            let mem_bmp = CreateCompatibleBitmap(hdc, width, height);
            let old_bmp = SelectObject(mem_dc, mem_bmp);

            // Paint background
            let bg_brush = CreateSolidBrush(COLORREF(COLOR_BG));
            FillRect(mem_dc, &client_rect, bg_brush);
            DeleteObject(bg_brush);

            if let Some(ref state) = APP_STATE {
                render_gui(mem_dc, width, height, state);
            }

            BitBlt(hdc, 0, 0, width, height, mem_dc, 0, 0, SRCCOPY);

            SelectObject(mem_dc, old_bmp);
            DeleteObject(mem_bmp);
            DeleteDC(mem_dc);

            EndPaint(hwnd, &ps);
            LRESULT(0)
        }

        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn make_font(height: i32, weight: i32, face: PCWSTR) -> HFONT {
    CreateFontW(
        height, 0, 0, 0, weight, 0, 0, 0,
        DEFAULT_CHARSET.0 as u32,
        OUT_DEFAULT_PRECIS.0 as u32,
        CLIP_DEFAULT_PRECIS.0 as u32,
        CLEARTYPE_QUALITY.0 as u32,
        DEFAULT_PITCH.0 as u32,
        face,
    )
}

unsafe fn draw_rounded_card(hdc: HDC, rect: &RECT, bg_color: u32, border_color: u32) {
    let brush = CreateSolidBrush(COLORREF(bg_color));
    let pen = CreatePen(PS_SOLID, 1, COLORREF(border_color));
    let old_brush = SelectObject(hdc, brush);
    let old_pen = SelectObject(hdc, pen);

    RoundRect(hdc, rect.left, rect.top, rect.right, rect.bottom, 12, 12);

    SelectObject(hdc, old_brush);
    SelectObject(hdc, old_pen);
    DeleteObject(brush);
    DeleteObject(pen);
}

unsafe fn draw_button(hdc: HDC, rect: &RECT, text: &str, bg_color: u32, text_color: u32) {
    draw_rounded_card(hdc, rect, bg_color, bg_color);
    SetBkMode(hdc, TRANSPARENT);
    SetTextColor(hdc, COLORREF(text_color));

    let mut w_text: Vec<u16> = text.encode_utf16().collect();
    let mut text_rect = *rect;
    DrawTextW(
        hdc,
        &mut w_text,
        &mut text_rect,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE,
    );
}

unsafe fn render_gui(hdc: HDC, _width: i32, _height: i32, state: &AppState) {
    SetBkMode(hdc, TRANSPARENT);

    // 1. Header Area
    // Logo and Title
    let font_title = make_font(26, FW_BOLD.0 as i32, w!("Segoe UI"));
    let old_font = SelectObject(hdc, font_title);
    SetTextColor(hdc, COLORREF(COLOR_TEXT_WHITE));
    let mut title_rect = RECT { left: 25, top: 15, right: 300, bottom: 45 };
    let mut title_str: Vec<u16> = "VrV Desk".encode_utf16().collect();
    DrawTextW(hdc, &mut title_str, &mut title_rect, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

    let font_subtitle = make_font(14, FW_NORMAL.0 as i32, w!("Segoe UI"));
    SelectObject(hdc, font_subtitle);
    SetTextColor(hdc, COLORREF(COLOR_TEXT_MUTED));
    let mut sub_rect = RECT { left: 25, top: 45, right: 450, bottom: 65 };
    let mut sub_str: Vec<u16> = "Ultra-low latency Remote Desktop & Screen Mirror".encode_utf16().collect();
    DrawTextW(hdc, &mut sub_str, &mut sub_rect, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

    // Header Status Pill
    let is_conn = state.telemetry.is_connected.load(Ordering::Relaxed);
    let is_sharing = state.is_sharing.load(Ordering::Relaxed);
    let (pill_text, pill_bg, pill_fg) = if is_conn {
        ("● CONNECTED", 0x00332210, COLOR_ACCENT_CYAN)
    } else if is_sharing {
        ("● READY FOR CONNECTIONS", 0x001B2B15, COLOR_ACCENT_GREEN)
    } else {
        ("● SHARING PAUSED", 0x001A1A33, COLOR_ACCENT_RED)
    };
    let pill_rect = RECT { left: 630, top: 22, right: 835, bottom: 52 };
    draw_button(hdc, &pill_rect, pill_text, pill_bg, pill_fg);

    // 2. Main Action Cards
    // Left Card: This Device (Host)
    let card_host = RECT { left: 25, top: 85, right: 535, bottom: 310 };
    draw_rounded_card(hdc, &card_host, COLOR_CARD, COLOR_CARD_BORDER);

    let font_bold = make_font(16, FW_SEMIBOLD.0 as i32, w!("Segoe UI"));
    SelectObject(hdc, font_bold);
    SetTextColor(hdc, COLORREF(COLOR_TEXT_WHITE));
    let mut host_header_rect = RECT { left: 45, top: 100, right: 500, bottom: 120 };
    let mut host_header: Vec<u16> = "Your Computer (This Device)".encode_utf16().collect();
    DrawTextW(hdc, &mut host_header, &mut host_header_rect, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

    // Device ID Label & Large Number
    SelectObject(hdc, font_subtitle);
    SetTextColor(hdc, COLORREF(COLOR_TEXT_MUTED));
    let mut id_lbl_rect = RECT { left: 45, top: 130, right: 300, bottom: 148 };
    let mut id_lbl: Vec<u16> = "Your 6-Digit Device ID".encode_utf16().collect();
    DrawTextW(hdc, &mut id_lbl, &mut id_lbl_rect, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

    let font_giant = make_font(32, FW_BOLD.0 as i32, w!("Segoe UI"));
    SelectObject(hdc, font_giant);
    SetTextColor(hdc, COLORREF(COLOR_ACCENT_CYAN));
    let formatted_id = format_six_digit_display(&state.device_id);
    let mut id_val_rect = RECT { left: 45, top: 145, right: 350, bottom: 185 };
    let mut id_val: Vec<u16> = formatted_id.encode_utf16().collect();
    DrawTextW(hdc, &mut id_val, &mut id_val_rect, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

    // Copy ID Button
    let copy_bg = if state.hover_button == Some(BTN_COPY_ID) { 0x003D2E24 } else { 0x002B211A };
    draw_button(hdc, &RECT_BTN_COPY_ID, "Copy ID", copy_bg, COLOR_ACCENT_CYAN);

    // PIN Label & Value
    SelectObject(hdc, font_subtitle);
    SetTextColor(hdc, COLORREF(COLOR_TEXT_MUTED));
    let mut pin_lbl_rect = RECT { left: 45, top: 190, right: 300, bottom: 208 };
    let mut pin_lbl: Vec<u16> = "One-Time Dynamic PIN".encode_utf16().collect();
    DrawTextW(hdc, &mut pin_lbl, &mut pin_lbl_rect, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

    let current_pin = { state.pin.read().unwrap().clone() };
    let formatted_pin = format_six_digit_display(&current_pin);
    let font_pin = make_font(22, FW_BOLD.0 as i32, w!("Segoe UI"));
    SelectObject(hdc, font_pin);
    SetTextColor(hdc, COLORREF(COLOR_TEXT_WHITE));
    let mut pin_val_rect = RECT { left: 45, top: 205, right: 350, bottom: 240 };
    let mut pin_val: Vec<u16> = formatted_pin.encode_utf16().collect();
    DrawTextW(hdc, &mut pin_val, &mut pin_val_rect, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

    // New PIN Button
    let new_pin_bg = if state.hover_button == Some(BTN_NEW_PIN) { 0x003D2E24 } else { 0x002B211A };
    draw_button(hdc, &RECT_BTN_NEW_PIN, "New PIN", new_pin_bg, COLOR_TEXT_WHITE);

    // Toggle Sharing Button
    let (toggle_text, toggle_bg, toggle_fg) = if is_sharing {
        ("■ Stop Sharing", 0x00222238, COLOR_ACCENT_RED)
    } else {
        ("▶ Start Sharing", 0x001B2B15, COLOR_ACCENT_GREEN)
    };
    draw_button(hdc, &RECT_BTN_TOGGLE_HOST, toggle_text, toggle_bg, toggle_fg);

    // Unattended Access Button
    let is_unattended = state.unattended.read().unwrap().enabled;
    let (unattended_text, unattended_bg, unattended_fg) = if is_unattended {
        ("🔑 Unattended: ON", 0x001B2B15, COLOR_ACCENT_GREEN)
    } else {
        ("🔒 Unattended: OFF", if state.hover_button == Some(BTN_UNATTENDED) { 0x003D2E24 } else { 0x002B211A }, COLOR_TEXT_MUTED)
    };
    draw_button(hdc, &RECT_BTN_UNATTENDED, unattended_text, unattended_bg, unattended_fg);

    // Right Card: Connect to Remote
    let card_remote = RECT { left: 545, top: 85, right: 855, bottom: 310 };
    draw_rounded_card(hdc, &card_remote, COLOR_CARD, COLOR_CARD_BORDER);

    SelectObject(hdc, font_bold);
    SetTextColor(hdc, COLORREF(COLOR_TEXT_WHITE));
    let mut rem_header_rect = RECT { left: 565, top: 100, right: 830, bottom: 120 };
    let mut rem_header: Vec<u16> = "Connect to Remote Device".encode_utf16().collect();
    DrawTextW(hdc, &mut rem_header, &mut rem_header_rect, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

    SelectObject(hdc, font_subtitle);
    SetTextColor(hdc, COLORREF(COLOR_TEXT_MUTED));
    let mut rem_id_rect = RECT { left: 565, top: 130, right: 830, bottom: 148 };
    let mut rem_id_lbl: Vec<u16> = "Remote Device ID or IP".encode_utf16().collect();
    DrawTextW(hdc, &mut rem_id_lbl, &mut rem_id_rect, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

    let mut rem_pin_rect = RECT { left: 565, top: 190, right: 830, bottom: 208 };
    let mut rem_pin_lbl: Vec<u16> = "Remote PIN".encode_utf16().collect();
    DrawTextW(hdc, &mut rem_pin_lbl, &mut rem_pin_rect, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

    // Connect Button
    let conn_bg = if state.hover_button == Some(BTN_CONNECT) { 0x00C89E28 } else { 0x00E2B838 };
    draw_button(hdc, &RECT_BTN_CONNECT, "Connect to Remote", conn_bg, 0x000F1217);

    // 3. Middle Section: Hardware & Engine Status Cards
    let status_card = RECT { left: 25, top: 320, right: 855, bottom: 385 };
    draw_rounded_card(hdc, &status_card, COLOR_CARD, COLOR_CARD_BORDER);

    SelectObject(hdc, font_subtitle);
    SetTextColor(hdc, COLORREF(COLOR_TEXT_MUTED));
    let mut stats: Vec<u16> = format!(
        "Capture: DXGI Desktop Duplication (GPU)   •   Video: MFT Hardware H.264 (NVENC/QSV/AMF)\nAudio: WASAPI Loopback 48kHz Stereo Opus   •   Network: LAN UDP :53210 & WebSocket :53211"
    ).encode_utf16().collect();
    let mut stats_rect = RECT { left: 45, top: 332, right: 835, bottom: 375 };
    DrawTextW(hdc, &mut stats, &mut stats_rect, DT_LEFT | DT_WORDBREAK);

    // 4. Bottom Section: Live Telemetry & Log Console
    let log_card = RECT { left: 25, top: 395, right: 855, bottom: 585 };
    draw_rounded_card(hdc, &log_card, COLOR_CARD, COLOR_CARD_BORDER);

    SelectObject(hdc, font_bold);
    SetTextColor(hdc, COLORREF(COLOR_TEXT_WHITE));
    let mut log_header_rect = RECT { left: 45, top: 408, right: 400, bottom: 428 };
    let mut log_header: Vec<u16> = "Live Session Activity & Telemetry".encode_utf16().collect();
    DrawTextW(hdc, &mut log_header, &mut log_header_rect, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

    // Telemetry Metrics
    let fps = state.telemetry.fps.load(Ordering::Relaxed);
    let kbps = state.telemetry.bitrate_kbps.load(Ordering::Relaxed);
    let sent = state.telemetry.frames_sent.load(Ordering::Relaxed);
    let skipped = state.telemetry.frames_skipped.load(Ordering::Relaxed);

    SelectObject(hdc, font_subtitle);
    SetTextColor(hdc, COLORREF(COLOR_ACCENT_CYAN));
    let mut metrics_str: Vec<u16> = format!(
        "FPS: {}   •   Bitrate: {:.1} Mbps   •   Frames Sent: {}   •   Static Skipped: {}",
        fps, kbps as f32 / 1000.0, sent, skipped
    ).encode_utf16().collect();
    let mut metrics_rect = RECT { left: 320, top: 408, right: 735, bottom: 428 };
    DrawTextW(hdc, &mut metrics_str, &mut metrics_rect, DT_RIGHT | DT_VCENTER | DT_SINGLELINE);

    // Clear Log button
    let clear_bg = if state.hover_button == Some(BTN_CLEAR_LOG) { 0x003D2E24 } else { 0x002B211A };
    draw_button(hdc, &RECT_BTN_CLEAR_LOG, "Clear", clear_bg, COLOR_TEXT_MUTED);

    // Console Log Box
    let console_rect = RECT { left: 45, top: 435, right: 835, bottom: 570 };
    draw_rounded_card(hdc, &console_rect, COLOR_CONSOLE_BG, COLOR_CARD_BORDER);

    if let Ok(logs) = state.telemetry.log_messages.read() {
        let display_lines: Vec<&String> = logs.iter().rev().take(6).collect();
        let font_console = make_font(13, FW_NORMAL.0 as i32, w!("Consolas"));
        SelectObject(hdc, font_console);
        SetTextColor(hdc, COLORREF(0x00A0D080)); // Soft terminal green

        let mut y_offset = 442;
        for line in display_lines.iter().rev() {
            let mut line_w: Vec<u16> = line.encode_utf16().collect();
            let mut line_rect = RECT { left: 55, top: y_offset, right: 825, bottom: y_offset + 18 };
            DrawTextW(hdc, &mut line_w, &mut line_rect, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
            y_offset += 20;
        }
        DeleteObject(font_console);
    }

    // 5. Footer
    SelectObject(hdc, font_subtitle);
    SetTextColor(hdc, COLORREF(0x0064748B));
    let mut footer_str: Vec<u16> = "VrV Desk v0.14.0 • ChaCha20-Poly1305 E2EE • Zero-Config LAN Multicast • Windows x64 Native".encode_utf16().collect();
    let mut footer_rect = RECT { left: 25, top: 595, right: 855, bottom: 615 };
    DrawTextW(hdc, &mut footer_str, &mut footer_rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

    // Clean up fonts
    SelectObject(hdc, old_font);
    DeleteObject(font_title);
    DeleteObject(font_subtitle);
    DeleteObject(font_bold);
    DeleteObject(font_giant);
    DeleteObject(font_pin);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pin_str = get_or_generate_pin(None);
    let pin = Arc::new(std::sync::RwLock::new(pin_str));
    let unattended = Arc::new(std::sync::RwLock::new(mirror_core::unattended::UnattendedConfig::load()));
    let device_id = get_or_generate_device_id(None);
    let host_name = get_host_name();
    let telemetry = HostTelemetry::default();
    let is_sharing = Arc::new(AtomicBool::new(true));

    let server_pin = pin.clone();
    let server_unattended = unattended.clone();
    let server_device_id = device_id.clone();
    let server_host_name = host_name.clone();
    let server_telem = telemetry.clone();

    // Spawn Background Tokio Runtime for VrV Desk Host Engine
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async move {
            let signal_url = std::env::var("VRV_SIGNAL_URL")
                .ok()
                .or_else(|| Some("ws://127.0.0.1:53212".to_string()));

            let _ = run_host_server_loop(
                server_device_id,
                server_pin,
                server_host_name,
                53211,
                signal_url,
                server_telem,
                Some(server_unattended),
            )
            .await;
        });
    });

    unsafe {
        APP_STATE = Some(AppState {
            device_id,
            pin,
            unattended,
            host_name,
            telemetry,
            is_sharing,
            hover_button: None,
            hwnd: HWND::default(),
            edit_remote_id: HWND::default(),
            edit_remote_pin: HWND::default(),
        });

        let hinstance = GetModuleHandleW(None)?;
        let class_name = w!("VrVDeskWindowClass");

        let wnd_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hbrBackground: HBRUSH(COLOR_WINDOW.0 as _),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };

        RegisterClassExW(&wnd_class);

        // Center on screen
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let win_w = 890;
        let win_h = 670;
        let x = (screen_w - win_w) / 2;
        let y = (screen_h - win_h) / 2;

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            w!("VrV Desk — Remote Desktop & Screen Mirror"),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            x,
            y,
            win_w,
            win_h,
            None,
            None,
            hinstance,
            None,
        );

        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    Ok(())
}
