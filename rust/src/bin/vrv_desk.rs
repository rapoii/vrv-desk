#![windows_subsystem = "windows"]

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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

// ============================================================================
// Neobrutalist Design Tokens (COLORREF format: 0x00BBGGRR)
// ============================================================================
const COLOR_INK: u32 = 0x00111111; // #111111 - Hard black outlines & text
const COLOR_PAPER: u32 = 0x00E8F7FF; // #FFF7E8 - Warm creamy background
const COLOR_WHITE: u32 = 0x00FFFFFF; // #FFFFFF - Card surface
const COLOR_YELLOW: u32 = 0x0047D4FF; // #FFD447 - Primary accent
const COLOR_CYAN: u32 = 0x00FFD670; // #70D6FF - Secondary accent
const COLOR_PINK: u32 = 0x00A670FF; // #FF70A6 - Accent badges / highlights
const COLOR_MINT: u32 = 0x00A8F17B; // #7BF1A8 - Success / Active accent
const COLOR_DANGER: u32 = 0x005C5CFF; // #FF5C5C - Warning / Stop accent
const COLOR_MUTED: u32 = 0x005B5B5B; // #5B5B5B - Secondary text

struct AppState {
    device_id: String,
    pin: Arc<std::sync::RwLock<String>>,
    unattended: Arc<std::sync::RwLock<mirror_core::unattended::UnattendedConfig>>,
    is_elevated: bool,
    _host_name: String,
    telemetry: HostTelemetry,
    is_sharing: Arc<AtomicBool>,
    hover_button: Option<u32>,
    hwnd: HWND,
    edit_remote_id: HWND,
    edit_remote_pin: HWND,
}

static mut APP_STATE: Option<AppState> = None;
static EDIT_BG_BRUSH: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

// Button IDs for hit testing
const BTN_COPY_ID: u32 = 1;
const BTN_NEW_PIN: u32 = 2;
const BTN_TOGGLE_HOST: u32 = 3;
const BTN_CONNECT: u32 = 4;
const BTN_UNATTENDED: u32 = 5;
const BTN_ELEVATE: u32 = 6;

// Button Rectangles (Simple beginner-friendly neobrutalist layout)
const RECT_BTN_COPY_ID: RECT = RECT {
    left: 305,
    top: 162,
    right: 415,
    bottom: 206,
};
const RECT_BTN_NEW_PIN: RECT = RECT {
    left: 305,
    top: 238,
    right: 415,
    bottom: 282,
};
const RECT_BTN_TOGGLE_HOST: RECT = RECT {
    left: 45,
    top: 300,
    right: 415,
    bottom: 352,
};
const RECT_BTN_UNATTENDED: RECT = RECT {
    left: 45,
    top: 366,
    right: 415,
    bottom: 418,
};
const RECT_BTN_CONNECT: RECT = RECT {
    left: 475,
    top: 300,
    right: 845,
    bottom: 362,
};
const RECT_BTN_ELEVATE: RECT = RECT {
    left: 480,
    top: 16,
    right: 615,
    bottom: 48,
};

fn point_in_rect(pt: POINT, r: RECT) -> bool {
    pt.x >= r.left && pt.x <= r.right && pt.y >= r.top && pt.y <= r.bottom
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            // Set Windows 11 Light Mode Title Bar to match creamy paper background
            let dark_mode: BOOL = BOOL(0);
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
                475,
                162,
                370,
                44,
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
                475,
                238,
                370,
                44,
                hwnd,
                HMENU(102 as _),
                hinstance,
                None,
            );

            // Set bold clean Segoe UI font on native edit controls
            let edit_font = make_font(18, FW_BOLD.0 as i32, w!("Segoe UI"));
            let _ = SendMessageW(edit_id, WM_SETFONT, WPARAM(edit_font.0 as _), LPARAM(1));
            let _ = SendMessageW(edit_pin, WM_SETFONT, WPARAM(edit_font.0 as _), LPARAM(1));

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
            } else if point_in_rect(pt, RECT_BTN_ELEVATE) {
                new_hover = Some(BTN_ELEVATE);
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
                    state
                        .telemetry
                        .add_log(format!("Device ID ({}) copied to clipboard!", id_clean));
                    InvalidateRect(hwnd, None, BOOL(0));
                } else if point_in_rect(pt, RECT_BTN_NEW_PIN) {
                    let new_pin = get_or_generate_pin(None);
                    *state.pin.write().unwrap() = new_pin.clone();
                    state.telemetry.add_log(format!(
                        "New PIN generated: {}",
                        format_six_digit_display(&new_pin)
                    ));
                    InvalidateRect(hwnd, None, BOOL(0));
                } else if point_in_rect(pt, RECT_BTN_TOGGLE_HOST) {
                    let current = state.is_sharing.load(Ordering::Relaxed);
                    state.is_sharing.store(!current, Ordering::Relaxed);
                    let status = if !current { "resumed" } else { "paused" };
                    state
                        .telemetry
                        .add_log(format!("Screen Sharing is now {}", status));
                    InvalidateRect(hwnd, None, BOOL(0));
                } else if point_in_rect(pt, RECT_BTN_UNATTENDED) {
                    let mut cfg = state.unattended.write().unwrap();
                    let new_state = !cfg.enabled;
                    cfg.enabled = new_state;
                    if new_state && cfg.password_hash.is_none() {
                        cfg.set_password("vrv2026");
                    }
                    let _ = cfg.save();
                    let status = if new_state {
                        "ENABLED (Pass: vrv2026)"
                    } else {
                        "DISABLED"
                    };
                    state
                        .telemetry
                        .add_log(format!("Unattended Access is now {}", status));
                    InvalidateRect(hwnd, None, BOOL(0));
                } else if point_in_rect(pt, RECT_BTN_CONNECT) {
                    state
                        .telemetry
                        .add_log("Connecting to remote device...".to_string());
                    InvalidateRect(hwnd, None, BOOL(0));
                } else if point_in_rect(pt, RECT_BTN_ELEVATE) {
                    if !state.is_elevated {
                        state
                            .telemetry
                            .add_log("Requesting Administrator UAC Elevation...".to_string());
                        let _ = mirror_core::service_manager::request_elevation(None, None);
                        let _ = PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
                    }
                }
            }

            LRESULT(0)
        }

        WM_CTLCOLOREDIT | WM_CTLCOLORSTATIC => {
            let hdc = HDC(wparam.0 as _);
            SetTextColor(hdc, COLORREF(COLOR_INK));
            SetBkColor(hdc, COLORREF(COLOR_WHITE));
            let brush_raw = EDIT_BG_BRUSH.load(Ordering::Relaxed);
            let brush = if brush_raw == 0 {
                let created = CreateSolidBrush(COLORREF(COLOR_WHITE));
                EDIT_BG_BRUSH.store(created.0 as isize, Ordering::Relaxed);
                created
            } else {
                HBRUSH(brush_raw as _)
            };
            LRESULT(brush.0 as _)
        }

        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);

            // Double buffering to eliminate all flicker
            let mut client_rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut client_rect);
            let width = client_rect.right - client_rect.left;
            let height = client_rect.bottom - client_rect.top;

            let mem_dc = CreateCompatibleDC(hdc);
            let mem_bmp = CreateCompatibleBitmap(hdc, width, height);
            let old_bmp = SelectObject(mem_dc, mem_bmp);

            // Paint background with paper color (#FFF7E8)
            let bg_brush = CreateSolidBrush(COLORREF(COLOR_PAPER));
            FillRect(mem_dc, &client_rect, bg_brush);
            DeleteObject(bg_brush);

            if let Some(ref state) = APP_STATE {
                render_gui(mem_dc, width, height, state);
            }

            let _ = BitBlt(hdc, 0, 0, width, height, mem_dc, 0, 0, SRCCOPY);

            SelectObject(mem_dc, old_bmp);
            DeleteObject(mem_bmp);
            DeleteDC(mem_dc);

            EndPaint(hwnd, &ps);
            LRESULT(0)
        }

        WM_DESTROY => {
            let brush_raw = EDIT_BG_BRUSH.swap(0, Ordering::Relaxed);
            if brush_raw != 0 {
                DeleteObject(HBRUSH(brush_raw as _));
            }
            PostQuitMessage(0);
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn make_font(height: i32, weight: i32, face: PCWSTR) -> HFONT {
    CreateFontW(
        height,
        0,
        0,
        0,
        weight,
        0,
        0,
        0,
        DEFAULT_CHARSET.0 as u32,
        OUT_DEFAULT_PRECIS.0 as u32,
        CLIP_DEFAULT_PRECIS.0 as u32,
        CLEARTYPE_QUALITY.0 as u32,
        DEFAULT_PITCH.0 as u32,
        face,
    )
}

/// Draws a Neobrutalist card with crisp ink outline and hard offset shadow (blur = 0).
unsafe fn draw_card(
    hdc: HDC,
    rect: &RECT,
    bg_color: u32,
    border_color: u32,
    border_width: i32,
    shadow_offset: i32,
    corner_radius: i32,
) {
    // 1. Draw hard shadow if offset > 0
    if shadow_offset > 0 {
        let shadow_brush = CreateSolidBrush(COLORREF(COLOR_INK));
        let shadow_pen = CreatePen(PS_SOLID, 1, COLORREF(COLOR_INK));
        let old_brush = SelectObject(hdc, shadow_brush);
        let old_pen = SelectObject(hdc, shadow_pen);

        RoundRect(
            hdc,
            rect.left + shadow_offset,
            rect.top + shadow_offset,
            rect.right + shadow_offset,
            rect.bottom + shadow_offset,
            corner_radius,
            corner_radius,
        );

        SelectObject(hdc, old_brush);
        SelectObject(hdc, old_pen);
        DeleteObject(shadow_brush);
        DeleteObject(shadow_pen);
    }

    // 2. Draw front card surface
    let fill_brush = CreateSolidBrush(COLORREF(bg_color));
    let border_pen = CreatePen(PS_INSIDEFRAME, border_width, COLORREF(border_color));
    let old_brush = SelectObject(hdc, fill_brush);
    let old_pen = SelectObject(hdc, border_pen);

    RoundRect(
        hdc,
        rect.left,
        rect.top,
        rect.right,
        rect.bottom,
        corner_radius,
        corner_radius,
    );

    SelectObject(hdc, old_brush);
    SelectObject(hdc, old_pen);
    DeleteObject(fill_brush);
    DeleteObject(border_pen);
}

/// Draws an interactive Neobrutalist button with physical tactile shift, outline, and accent fill.
unsafe fn draw_button(
    hdc: HDC,
    rect: &RECT,
    text: &str,
    bg_color: u32,
    hover_bg_color: u32,
    text_color: u32,
    is_hovered: bool,
    font: HFONT,
) {
    // Physical interaction:
    // Rest: face at (0, 0), shadow at (+4, +4)
    // Hover: face shifts (+2, +2) towards shadow, remaining shadow is 2px, accent fill
    let (shift_x, shift_y, shadow_offset, fill_color) = if is_hovered {
        (2, 2, 2, hover_bg_color)
    } else {
        (0, 0, 4, bg_color)
    };

    let face_rect = RECT {
        left: rect.left + shift_x,
        top: rect.top + shift_y,
        right: rect.right + shift_x,
        bottom: rect.bottom + shift_y,
    };

    // Draw shadow
    if shadow_offset > 0 {
        let shadow_brush = CreateSolidBrush(COLORREF(COLOR_INK));
        let shadow_pen = CreatePen(PS_SOLID, 1, COLORREF(COLOR_INK));
        let old_brush = SelectObject(hdc, shadow_brush);
        let old_pen = SelectObject(hdc, shadow_pen);

        RoundRect(
            hdc,
            face_rect.left + shadow_offset,
            face_rect.top + shadow_offset,
            face_rect.right + shadow_offset,
            face_rect.bottom + shadow_offset,
            6,
            6,
        );

        SelectObject(hdc, old_brush);
        SelectObject(hdc, old_pen);
        DeleteObject(shadow_brush);
        DeleteObject(shadow_pen);
    }

    // Draw face
    let fill_brush = CreateSolidBrush(COLORREF(fill_color));
    let border_pen = CreatePen(PS_INSIDEFRAME, 3, COLORREF(COLOR_INK));
    let old_brush = SelectObject(hdc, fill_brush);
    let old_pen = SelectObject(hdc, border_pen);

    RoundRect(
        hdc,
        face_rect.left,
        face_rect.top,
        face_rect.right,
        face_rect.bottom,
        6,
        6,
    );

    SelectObject(hdc, old_brush);
    SelectObject(hdc, old_pen);
    DeleteObject(fill_brush);
    DeleteObject(border_pen);

    // Draw text inside face
    SetBkMode(hdc, TRANSPARENT);
    SetTextColor(hdc, COLORREF(text_color));
    let old_font = SelectObject(hdc, font);

    let mut w_text: Vec<u16> = text.encode_utf16().collect();
    let mut text_rect = face_rect;
    DrawTextW(
        hdc,
        &mut w_text,
        &mut text_rect,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE,
    );

    SelectObject(hdc, old_font);
}

/// Draws a Neobrutalist status pill badge (outline + shadow + explicit badge text).
unsafe fn draw_status_pill(
    hdc: HDC,
    rect: &RECT,
    text: &str,
    bg_color: u32,
    text_color: u32,
    font: HFONT,
) {
    draw_card(hdc, rect, bg_color, COLOR_INK, 2, 0, 6);

    SetBkMode(hdc, TRANSPARENT);
    SetTextColor(hdc, COLORREF(text_color));
    let old_font = SelectObject(hdc, font);

    let mut w_text: Vec<u16> = text.encode_utf16().collect();
    let mut text_rect = *rect;
    DrawTextW(
        hdc,
        &mut w_text,
        &mut text_rect,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE,
    );

    SelectObject(hdc, old_font);
}

unsafe fn render_gui(hdc: HDC, _width: i32, _height: i32, state: &AppState) {
    SetBkMode(hdc, TRANSPARENT);

    // Typography hierarchy using crisp system fonts
    let font_title = make_font(26, 900, w!("Arial Black"));
    let font_badge = make_font(11, FW_BOLD.0 as i32, w!("Segoe UI"));
    let font_subtitle = make_font(12, FW_BOLD.0 as i32, w!("Segoe UI"));
    let font_card_header = make_font(16, 900, w!("Arial Black"));
    let font_card_subtitle = make_font(12, FW_NORMAL.0 as i32, w!("Segoe UI"));
    let font_label = make_font(11, FW_BOLD.0 as i32, w!("Segoe UI"));
    let font_giant_id = make_font(26, 900, w!("Arial Black"));
    let font_giant_pin = make_font(24, 900, w!("Arial Black"));
    let font_btn = make_font(13, FW_BOLD.0 as i32, w!("Segoe UI"));
    let font_btn_action = make_font(14, 900, w!("Arial Black"));
    let font_btn_lg = make_font(16, 900, w!("Arial Black"));
    let font_tip = make_font(11, FW_NORMAL.0 as i32, w!("Segoe UI"));
    let font_footer = make_font(11, FW_BOLD.0 as i32, w!("Segoe UI"));

    // ------------------------------------------------------------------------
    // 1. Header Area
    // ------------------------------------------------------------------------
    let old_font = SelectObject(hdc, font_title);
    SetTextColor(hdc, COLORREF(COLOR_INK));
    let mut title_rect = RECT {
        left: 25,
        top: 14,
        right: 185,
        bottom: 44,
    };
    let mut title_str: Vec<u16> = "VRV DESK".encode_utf16().collect();
    DrawTextW(
        hdc,
        &mut title_str,
        &mut title_rect,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE,
    );

    // Accent Edition Tag next to Title
    let tag_rect = RECT {
        left: 195,
        top: 18,
        right: 265,
        bottom: 40,
    };
    draw_card(hdc, &tag_rect, COLOR_YELLOW, COLOR_INK, 2, 0, 4);
    SelectObject(hdc, font_badge);
    SetTextColor(hdc, COLORREF(COLOR_INK));
    let mut tag_text: Vec<u16> = "v0.14.0".encode_utf16().collect();
    let mut tag_draw_rect = tag_rect;
    DrawTextW(
        hdc,
        &mut tag_text,
        &mut tag_draw_rect,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE,
    );

    // Subtitle
    SelectObject(hdc, font_subtitle);
    SetTextColor(hdc, COLORREF(COLOR_MUTED));
    let mut sub_rect = RECT {
        left: 25,
        top: 44,
        right: 470,
        bottom: 64,
    };
    let mut sub_str: Vec<u16> = "Simple remote help — share your numbers or connect to a friend"
        .encode_utf16()
        .collect();
    DrawTextW(
        hdc,
        &mut sub_str,
        &mut sub_rect,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE,
    );

    // Admin / Elevation Pill Button
    let (elev_text, elev_bg, elev_hover) = if state.is_elevated {
        ("[ADMIN] ELEVATED", COLOR_MINT, COLOR_CYAN)
    } else {
        ("[UAC] ELEVATE", COLOR_YELLOW, COLOR_CYAN)
    };
    let is_elev_hover = state.hover_button == Some(BTN_ELEVATE);
    draw_button(
        hdc,
        &RECT_BTN_ELEVATE,
        elev_text,
        elev_bg,
        elev_hover,
        COLOR_INK,
        is_elev_hover,
        font_btn,
    );

    // Header Status Pill
    let is_conn = state.telemetry.is_connected.load(Ordering::Relaxed);
    let is_sharing = state.is_sharing.load(Ordering::Relaxed);
    let (pill_text, pill_bg) = if is_conn {
        ("[LIVE] CONNECTED", COLOR_CYAN)
    } else if is_sharing {
        ("[READY] SHARING ACTIVE", COLOR_MINT)
    } else {
        ("[PAUSED] SHARING STOPPED", COLOR_DANGER)
    };
    let pill_rect = RECT {
        left: 625,
        top: 16,
        right: 865,
        bottom: 48,
    };
    draw_status_pill(hdc, &pill_rect, pill_text, pill_bg, COLOR_INK, font_btn);

    // ------------------------------------------------------------------------
    // 2. Left Card: THIS COMPUTER (HOST)
    // ------------------------------------------------------------------------
    let card_host = RECT {
        left: 25,
        top: 72,
        right: 435,
        bottom: 438,
    };
    draw_card(hdc, &card_host, COLOR_WHITE, COLOR_INK, 3, 4, 6);

    // Header
    SelectObject(hdc, font_card_header);
    SetTextColor(hdc, COLORREF(COLOR_INK));
    let mut host_header_rect = RECT {
        left: 45,
        top: 86,
        right: 415,
        bottom: 110,
    };
    let mut host_header: Vec<u16> = "THIS COMPUTER (HOST)".encode_utf16().collect();
    DrawTextW(
        hdc,
        &mut host_header,
        &mut host_header_rect,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE,
    );

    // Subtitle
    SelectObject(hdc, font_card_subtitle);
    SetTextColor(hdc, COLORREF(COLOR_MUTED));
    let mut host_sub_rect = RECT {
        left: 45,
        top: 110,
        right: 415,
        bottom: 130,
    };
    let mut host_sub: Vec<u16> = "Share your ID & PIN to let someone help you:"
        .encode_utf16()
        .collect();
    DrawTextW(
        hdc,
        &mut host_sub,
        &mut host_sub_rect,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE,
    );

    // Label ID
    SelectObject(hdc, font_label);
    SetTextColor(hdc, COLORREF(COLOR_MUTED));
    let mut id_lbl_rect = RECT {
        left: 45,
        top: 140,
        right: 295,
        bottom: 158,
    };
    let mut id_lbl: Vec<u16> = "YOUR 6-DIGIT DEVICE ID".encode_utf16().collect();
    DrawTextW(
        hdc,
        &mut id_lbl,
        &mut id_lbl_rect,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE,
    );

    // ID Container Box (Flat yellow paper, crisp border, zero shadow)
    let id_box_rect = RECT {
        left: 45,
        top: 162,
        right: 295,
        bottom: 206,
    };
    draw_card(hdc, &id_box_rect, COLOR_PAPER, COLOR_INK, 2, 0, 4);

    SelectObject(hdc, font_giant_id);
    SetTextColor(hdc, COLORREF(COLOR_INK));
    let formatted_id = format_six_digit_display(&state.device_id);
    let mut id_val_rect = RECT {
        left: 55,
        top: 162,
        right: 285,
        bottom: 206,
    };
    let mut id_val: Vec<u16> = formatted_id.encode_utf16().collect();
    DrawTextW(
        hdc,
        &mut id_val,
        &mut id_val_rect,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE,
    );

    // Copy ID Button
    let is_copy_hover = state.hover_button == Some(BTN_COPY_ID);
    draw_button(
        hdc,
        &RECT_BTN_COPY_ID,
        "COPY ID",
        COLOR_CYAN,
        COLOR_YELLOW,
        COLOR_INK,
        is_copy_hover,
        font_btn,
    );

    // Label PIN
    SelectObject(hdc, font_label);
    SetTextColor(hdc, COLORREF(COLOR_MUTED));
    let mut pin_lbl_rect = RECT {
        left: 45,
        top: 216,
        right: 295,
        bottom: 234,
    };
    let mut pin_lbl: Vec<u16> = "ONE-TIME ACCESS PIN".encode_utf16().collect();
    DrawTextW(
        hdc,
        &mut pin_lbl,
        &mut pin_lbl_rect,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE,
    );

    // PIN Container Box (Flat white, crisp border, zero shadow)
    let pin_box_rect = RECT {
        left: 45,
        top: 238,
        right: 295,
        bottom: 282,
    };
    draw_card(hdc, &pin_box_rect, COLOR_PAPER, COLOR_INK, 2, 0, 4);

    let current_pin = { state.pin.read().unwrap().clone() };
    let formatted_pin = format_six_digit_display(&current_pin);
    SelectObject(hdc, font_giant_pin);
    SetTextColor(hdc, COLORREF(COLOR_INK));
    let mut pin_val_rect = RECT {
        left: 55,
        top: 238,
        right: 285,
        bottom: 282,
    };
    let mut pin_val: Vec<u16> = formatted_pin.encode_utf16().collect();
    DrawTextW(
        hdc,
        &mut pin_val,
        &mut pin_val_rect,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE,
    );

    // New PIN Button
    let is_new_pin_hover = state.hover_button == Some(BTN_NEW_PIN);
    draw_button(
        hdc,
        &RECT_BTN_NEW_PIN,
        "NEW PIN",
        COLOR_WHITE,
        COLOR_CYAN,
        COLOR_INK,
        is_new_pin_hover,
        font_btn,
    );

    // Toggle Sharing Button
    let (toggle_text, toggle_bg, toggle_hover, toggle_fg) = if is_sharing {
        ("STOP SHARING", COLOR_DANGER, COLOR_PINK, COLOR_INK)
    } else {
        ("START SHARING", COLOR_MINT, COLOR_YELLOW, COLOR_INK)
    };
    let is_toggle_hover = state.hover_button == Some(BTN_TOGGLE_HOST);
    draw_button(
        hdc,
        &RECT_BTN_TOGGLE_HOST,
        toggle_text,
        toggle_bg,
        toggle_hover,
        toggle_fg,
        is_toggle_hover,
        font_btn_action,
    );

    // Unattended Access Button
    let is_unattended = state.unattended.read().unwrap().enabled;
    let (unattended_text, unattended_bg, unattended_hover) = if is_unattended {
        ("UNATTENDED ACCESS: ON", COLOR_MINT, COLOR_YELLOW)
    } else {
        ("UNATTENDED ACCESS: OFF", COLOR_WHITE, COLOR_CYAN)
    };
    let is_unattended_hover = state.hover_button == Some(BTN_UNATTENDED);
    draw_button(
        hdc,
        &RECT_BTN_UNATTENDED,
        unattended_text,
        unattended_bg,
        unattended_hover,
        COLOR_INK,
        is_unattended_hover,
        font_btn,
    );

    // ------------------------------------------------------------------------
    // 3. Right Card: CONNECT TO REMOTE (CLIENT)
    // ------------------------------------------------------------------------
    let card_remote = RECT {
        left: 455,
        top: 72,
        right: 865,
        bottom: 438,
    };
    draw_card(hdc, &card_remote, COLOR_WHITE, COLOR_INK, 3, 4, 6);

    // Header
    SelectObject(hdc, font_card_header);
    SetTextColor(hdc, COLORREF(COLOR_INK));
    let mut rem_header_rect = RECT {
        left: 475,
        top: 86,
        right: 845,
        bottom: 110,
    };
    let mut rem_header: Vec<u16> = "REMOTE CONTROL (CLIENT)".encode_utf16().collect();
    DrawTextW(
        hdc,
        &mut rem_header,
        &mut rem_header_rect,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE,
    );

    // Subtitle
    SelectObject(hdc, font_card_subtitle);
    SetTextColor(hdc, COLORREF(COLOR_MUTED));
    let mut rem_sub_rect = RECT {
        left: 475,
        top: 110,
        right: 845,
        bottom: 130,
    };
    let mut rem_sub: Vec<u16> = "Control a friend's PC by entering their ID & PIN:"
        .encode_utf16()
        .collect();
    DrawTextW(
        hdc,
        &mut rem_sub,
        &mut rem_sub_rect,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE,
    );

    // Remote ID Label
    SelectObject(hdc, font_label);
    SetTextColor(hdc, COLORREF(COLOR_MUTED));
    let mut rem_id_rect = RECT {
        left: 475,
        top: 140,
        right: 845,
        bottom: 158,
    };
    let mut rem_id_lbl: Vec<u16> = "REMOTE DEVICE ID OR IP".encode_utf16().collect();
    DrawTextW(
        hdc,
        &mut rem_id_lbl,
        &mut rem_id_rect,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE,
    );

    // Remote PIN Label
    let mut rem_pin_rect = RECT {
        left: 475,
        top: 216,
        right: 845,
        bottom: 234,
    };
    let mut rem_pin_lbl: Vec<u16> = "REMOTE ACCESS PIN".encode_utf16().collect();
    DrawTextW(
        hdc,
        &mut rem_pin_lbl,
        &mut rem_pin_rect,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE,
    );

    // Connect Button
    let is_conn_hover = state.hover_button == Some(BTN_CONNECT);
    draw_button(
        hdc,
        &RECT_BTN_CONNECT,
        "CONNECT TO REMOTE",
        COLOR_YELLOW,
        COLOR_MINT,
        COLOR_INK,
        is_conn_hover,
        font_btn_lg,
    );

    // Friendly helper tip
    SelectObject(hdc, font_tip);
    SetTextColor(hdc, COLORREF(COLOR_MUTED));
    let mut tip_rect = RECT {
        left: 475,
        top: 382,
        right: 845,
        bottom: 422,
    };
    let mut tip_text: Vec<u16> =
        "Tip: Ask your partner for their 6-digit Device ID and PIN shown on their VrV Desk screen."
            .encode_utf16()
            .collect();
    DrawTextW(hdc, &mut tip_text, &mut tip_rect, DT_LEFT | DT_WORDBREAK);

    // ------------------------------------------------------------------------
    // 4. Bottom Security & Performance Strip
    // ------------------------------------------------------------------------
    let footer_card = RECT {
        left: 25,
        top: 454,
        right: 865,
        bottom: 494,
    };
    draw_card(hdc, &footer_card, COLOR_PAPER, COLOR_INK, 2, 0, 4);

    SelectObject(hdc, font_footer);
    SetTextColor(hdc, COLORREF(COLOR_MUTED));
    let mut footer_str: Vec<u16> =
        "VrV Desk v0.14.0  •  ChaCha20-Poly1305 E2EE  •  Direct GPU Duplication 120 FPS  •  Zero Setup LAN Multicast"
            .encode_utf16()
            .collect();
    let mut footer_rect = footer_card;
    DrawTextW(
        hdc,
        &mut footer_str,
        &mut footer_rect,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE,
    );

    // Clean up fonts to prevent GDI resource leaks
    SelectObject(hdc, old_font);
    DeleteObject(font_title);
    DeleteObject(font_badge);
    DeleteObject(font_subtitle);
    DeleteObject(font_card_header);
    DeleteObject(font_card_subtitle);
    DeleteObject(font_label);
    DeleteObject(font_giant_id);
    DeleteObject(font_giant_pin);
    DeleteObject(font_btn);
    DeleteObject(font_btn_action);
    DeleteObject(font_btn_lg);
    DeleteObject(font_tip);
    DeleteObject(font_footer);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pin_str = get_or_generate_pin(None);
    let pin = Arc::new(std::sync::RwLock::new(pin_str));
    let unattended = Arc::new(std::sync::RwLock::new(
        mirror_core::unattended::UnattendedConfig::load(),
    ));
    let device_id = get_or_generate_device_id(None);
    let host_name = get_host_name();
    let is_elevated = mirror_core::service_manager::is_elevated();
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
            is_elevated,
            _host_name: host_name,
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
        let win_w = 900;
        let win_h = 560;
        let x = (screen_w - win_w) / 2;
        let y = (screen_h - win_h) / 2;

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            w!("VrV Desk — Remote Desktop"),
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
