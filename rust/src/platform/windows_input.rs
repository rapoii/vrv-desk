use crate::protocol::{InputEvent, MouseButton};
#[cfg(windows)]
use windows::Win32::Foundation::*;
#[cfg(windows)]
use windows::Win32::System::DataExchange::*;
#[cfg(windows)]
use windows::Win32::System::Memory::*;
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::*;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::SetCursorPos;

#[cfg(windows)]
static ACTIVE_MONITOR_BOUNDS: std::sync::RwLock<(i32, i32, u32, u32)> =
    std::sync::RwLock::new((0, 0, 0, 0));

#[cfg(windows)]
pub fn set_active_monitor_bounds(left: i32, top: i32, width: u32, height: u32) {
    if let Ok(mut lock) = ACTIVE_MONITOR_BOUNDS.write() {
        *lock = (left, top, width, height);
    }
}

#[cfg(windows)]
pub fn get_active_monitor_bounds() -> (i32, i32, u32, u32) {
    ACTIVE_MONITOR_BOUNDS
        .read()
        .map(|b| *b)
        .unwrap_or((0, 0, 0, 0))
}

#[cfg(windows)]
fn map_coords(x: f32, y: f32, fallback_w: u32, fallback_h: u32) -> (i32, i32) {
    let (ox, oy, mw, mh) = get_active_monitor_bounds();
    let (base_w, base_h) = if mw > 0 && mh > 0 {
        (mw, mh)
    } else {
        (fallback_w, fallback_h)
    };
    let px = ox + (x * base_w as f32) as i32;
    let py = oy + (y * base_h as f32) as i32;
    (px, py)
}

#[cfg(not(windows))]
pub fn set_active_monitor_bounds(_left: i32, _top: i32, _width: u32, _height: u32) {}

#[cfg(windows)]
pub fn inject_input(event: &InputEvent, _screen_w: u32, _screen_h: u32) -> Result<(), String> {
    unsafe {
        match event {
            InputEvent::MouseMove { x, y } => {
                let (px, py) = map_coords(*x, *y, _screen_w, _screen_h);
                let _ = SetCursorPos(px, py);
            }

            InputEvent::MouseDown { x: _, y: _, button } => {
                let flag = match button {
                    MouseButton::Left => MOUSEEVENTF_LEFTDOWN,
                    MouseButton::Right => MOUSEEVENTF_RIGHTDOWN,
                    MouseButton::Middle => MOUSEEVENTF_MIDDLEDOWN,
                };
                let input = INPUT {
                    r#type: INPUT_MOUSE,
                    Anonymous: INPUT_0 {
                        mi: MOUSEINPUT {
                            dx: 0,
                            dy: 0,
                            mouseData: 0,
                            dwFlags: flag,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
            }
            InputEvent::MouseUp { x: _, y: _, button } => {
                let flag = match button {
                    MouseButton::Left => MOUSEEVENTF_LEFTUP,
                    MouseButton::Right => MOUSEEVENTF_RIGHTUP,
                    MouseButton::Middle => MOUSEEVENTF_MIDDLEUP,
                };
                let input = INPUT {
                    r#type: INPUT_MOUSE,
                    Anonymous: INPUT_0 {
                        mi: MOUSEINPUT {
                            dx: 0,
                            dy: 0,
                            mouseData: 0,
                            dwFlags: flag,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
            }
            InputEvent::MouseWheel { delta_y } => {
                let mouse_data = (*delta_y as i32) * 120;
                let input = INPUT {
                    r#type: INPUT_MOUSE,
                    Anonymous: INPUT_0 {
                        mi: MOUSEINPUT {
                            dx: 0,
                            dy: 0,
                            mouseData: mouse_data as u32,
                            dwFlags: MOUSEEVENTF_WHEEL,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
            }
            InputEvent::KeyDown { keycode } => {
                let input = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(*keycode as u16),
                            wScan: 0,
                            dwFlags: KEYBD_EVENT_FLAGS(0),
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
            }
            InputEvent::KeyUp { keycode } => {
                let input = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(*keycode as u16),
                            wScan: 0,
                            dwFlags: KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
            }
            InputEvent::TouchTap { x, y } => {
                let (px, py) = map_coords(*x, *y, _screen_w, _screen_h);
                let _ = SetCursorPos(px, py);
                let down_input = INPUT {
                    r#type: INPUT_MOUSE,
                    Anonymous: INPUT_0 {
                        mi: MOUSEINPUT {
                            dx: 0,
                            dy: 0,
                            mouseData: 0,
                            dwFlags: MOUSEEVENTF_LEFTDOWN,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                let up_input = INPUT {
                    r#type: INPUT_MOUSE,
                    Anonymous: INPUT_0 {
                        mi: MOUSEINPUT {
                            dx: 0,
                            dy: 0,
                            mouseData: 0,
                            dwFlags: MOUSEEVENTF_LEFTUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                SendInput(&[down_input, up_input], std::mem::size_of::<INPUT>() as i32);
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn inject_input(_event: &InputEvent, _screen_w: u32, _screen_h: u32) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
pub fn inject_unicode_text(text: &str) -> Result<(), String> {
    unsafe {
        let mut inputs = Vec::new();
        for ch in text.encode_utf16() {
            // Key down with KEYEVENTF_UNICODE
            inputs.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0),
                        wScan: ch,
                        dwFlags: KEYEVENTF_UNICODE,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            });
            // Key up with KEYEVENTF_UNICODE
            inputs.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0),
                        wScan: ch,
                        dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            });
        }
        if !inputs.is_empty() {
            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn inject_unicode_text(_text: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
unsafe fn press_single_vk(vk: VIRTUAL_KEY) {
    let down = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: KEYBD_EVENT_FLAGS(0),
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let up = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    SendInput(&[down, up], std::mem::size_of::<INPUT>() as i32);
}

#[cfg(windows)]
unsafe fn press_combo_vk(mod_vk: VIRTUAL_KEY, key_vk: VIRTUAL_KEY) {
    let inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: mod_vk,
                    wScan: 0,
                    dwFlags: KEYBD_EVENT_FLAGS(0),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: key_vk,
                    wScan: 0,
                    dwFlags: KEYBD_EVENT_FLAGS(0),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: key_vk,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: mod_vk,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];
    SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
}

#[cfg(windows)]
pub fn inject_shortcut(name: &str) -> Result<(), String> {
    unsafe {
        match name {
            "win" => press_single_vk(VK_LWIN),
            "esc" => press_single_vk(VK_ESCAPE),
            "enter" => press_single_vk(VK_RETURN),
            "backspace" => press_single_vk(VK_BACK),
            "tab" => press_single_vk(VK_TAB),
            "space" => press_single_vk(VK_SPACE),
            "delete" | "del" => press_single_vk(VK_DELETE),
            "insert" | "ins" => press_single_vk(VK_INSERT),
            "home" | "pos1" => press_single_vk(VK_HOME),
            "end" => press_single_vk(VK_END),
            "page_up" | "pgup" => press_single_vk(VK_PRIOR),
            "page_down" | "pgdn" => press_single_vk(VK_NEXT),
            "prtscn" | "print_screen" | "sysrq" => press_single_vk(VK_SNAPSHOT),
            "up" | "arrow_up" => press_single_vk(VK_UP),
            "down" | "arrow_down" => press_single_vk(VK_DOWN),
            "left" | "arrow_left" => press_single_vk(VK_LEFT),
            "right" | "arrow_right" => press_single_vk(VK_RIGHT),
            "f1" => press_single_vk(VK_F1),
            "f2" => press_single_vk(VK_F2),
            "f3" => press_single_vk(VK_F3),
            "f4" => press_single_vk(VK_F4),
            "f5" => press_single_vk(VK_F5),
            "f6" => press_single_vk(VK_F6),
            "f7" => press_single_vk(VK_F7),
            "f8" => press_single_vk(VK_F8),
            "f9" => press_single_vk(VK_F9),
            "f10" => press_single_vk(VK_F10),
            "f11" => press_single_vk(VK_F11),
            "f12" => press_single_vk(VK_F12),
            "ctrl_c" => press_combo_vk(VK_CONTROL, VIRTUAL_KEY(0x43)),
            "ctrl_v" => press_combo_vk(VK_CONTROL, VIRTUAL_KEY(0x56)),
            "ctrl_x" => press_combo_vk(VK_CONTROL, VIRTUAL_KEY(0x58)),
            "ctrl_z" => press_combo_vk(VK_CONTROL, VIRTUAL_KEY(0x5A)),
            "ctrl_a" => press_combo_vk(VK_CONTROL, VIRTUAL_KEY(0x41)),
            "win_r" => press_combo_vk(VK_LWIN, VIRTUAL_KEY(0x52)),

            "task_manager" => {
                // Ctrl + Shift + Esc
                let make_key = |vk: VIRTUAL_KEY, up: bool| INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: vk,
                            wScan: 0,
                            dwFlags: if up {
                                KEYEVENTF_KEYUP
                            } else {
                                KEYBD_EVENT_FLAGS(0)
                            },
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                let inputs = [
                    make_key(VK_CONTROL, false),
                    make_key(VK_SHIFT, false),
                    make_key(VK_ESCAPE, false),
                    make_key(VK_ESCAPE, true),
                    make_key(VK_SHIFT, true),
                    make_key(VK_CONTROL, true),
                ];
                SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
            }
            "alt_tab" => {
                // Alt + Tab
                let make_key = |vk: VIRTUAL_KEY, up: bool| INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: vk,
                            wScan: 0,
                            dwFlags: if up {
                                KEYEVENTF_KEYUP
                            } else {
                                KEYBD_EVENT_FLAGS(0)
                            },
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                let inputs = [
                    make_key(VK_MENU, false),
                    make_key(VK_TAB, false),
                    make_key(VK_TAB, true),
                    make_key(VK_MENU, true),
                ];
                SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
            }
            "show_desktop" => {
                // Win + D (0x44)
                let make_key = |vk: VIRTUAL_KEY, up: bool| INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: vk,
                            wScan: 0,
                            dwFlags: if up {
                                KEYEVENTF_KEYUP
                            } else {
                                KEYBD_EVENT_FLAGS(0)
                            },
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                let inputs = [
                    make_key(VK_LWIN, false),
                    make_key(VIRTUAL_KEY(0x44), false),
                    make_key(VIRTUAL_KEY(0x44), true),
                    make_key(VK_LWIN, true),
                ];
                SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
            }
            "ctrl_alt_del" | "sas" => {
                let _ = crate::service_manager::trigger_sas();
            }
            "elevate" => {
                let _ = crate::service_manager::request_elevation(None, Some("--elevated"));
            }
            _ => return Err(format!("Unknown shortcut: {}", name)),
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn inject_shortcut(_name: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
pub fn get_clipboard_text() -> Option<String> {
    unsafe {
        if OpenClipboard(HWND(0)).is_err() {
            return None;
        }

        // CF_UNICODETEXT is standard Windows clipboard format 13
        const CF_UNICODETEXT: u32 = 13;
        let handle = GetClipboardData(CF_UNICODETEXT);
        if handle.is_err() {
            let _ = CloseClipboard();
            return None;
        }
        let handle = handle.unwrap();
        if handle.0 == 0 {
            let _ = CloseClipboard();
            return None;
        }

        let ptr = GlobalLock(HGLOBAL(handle.0 as *mut _));
        if ptr.is_null() {
            let _ = CloseClipboard();
            return None;
        }

        let mut len = 0;
        let wide_ptr = ptr as *const u16;
        while *wide_ptr.add(len) != 0 {
            len += 1;
        }

        let slice = std::slice::from_raw_parts(wide_ptr, len);
        let result = String::from_utf16_lossy(slice);

        let _ = GlobalUnlock(HGLOBAL(handle.0 as *mut _));
        let _ = CloseClipboard();

        Some(result)
    }
}

#[cfg(not(windows))]
pub fn get_clipboard_text() -> Option<String> {
    None
}

#[cfg(windows)]
pub fn set_clipboard_text(text: &str) -> bool {
    unsafe {
        if OpenClipboard(HWND(0)).is_err() {
            return false;
        }

        if EmptyClipboard().is_err() {
            let _ = CloseClipboard();
            return false;
        }

        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let bytes_len = wide.len() * std::mem::size_of::<u16>();

        let h_mem = GlobalAlloc(GMEM_MOVEABLE, bytes_len);
        if h_mem.is_err() {
            let _ = CloseClipboard();
            return false;
        }
        let h_mem = h_mem.unwrap();

        let ptr = GlobalLock(h_mem);
        if ptr.is_null() {
            let _ = GlobalFree(h_mem);
            let _ = CloseClipboard();
            return false;
        }

        std::ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, ptr as *mut u8, bytes_len);
        let _ = GlobalUnlock(h_mem);

        const CF_UNICODETEXT: u32 = 13;
        let handle = HANDLE(h_mem.0 as isize);
        if SetClipboardData(CF_UNICODETEXT, handle).is_err() {
            let _ = GlobalFree(h_mem);
            let _ = CloseClipboard();
            return false;
        }

        let _ = CloseClipboard();
        true
    }
}

#[cfg(not(windows))]
pub fn set_clipboard_text(_text: &str) -> bool {
    true
}

#[cfg(windows)]
pub fn get_clipboard_sequence_number() -> u32 {
    unsafe { windows::Win32::System::DataExchange::GetClipboardSequenceNumber() }
}

#[cfg(not(windows))]
pub fn get_clipboard_sequence_number() -> u32 {
    0
}
