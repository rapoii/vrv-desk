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
pub fn inject_input(event: &InputEvent, _screen_w: u32, _screen_h: u32) -> Result<(), String> {
    unsafe {
        match event {
            InputEvent::MouseMove { x, y } => {
                let px = (x * _screen_w as f32) as i32;
                let py = (y * _screen_h as f32) as i32;
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
                let px = (x * _screen_w as f32) as i32;
                let py = (y * _screen_h as f32) as i32;
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
pub fn inject_shortcut(name: &str) -> Result<(), String> {
    unsafe {
        match name {
            "win" => {
                let down = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VK_LWIN,
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
                            wVk: VK_LWIN,
                            wScan: 0,
                            dwFlags: KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                SendInput(&[down, up], std::mem::size_of::<INPUT>() as i32);
            }
            "esc" => {
                let down = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VK_ESCAPE,
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
                            wVk: VK_ESCAPE,
                            wScan: 0,
                            dwFlags: KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                SendInput(&[down, up], std::mem::size_of::<INPUT>() as i32);
            }
            "enter" => {
                let down = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VK_RETURN,
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
                            wVk: VK_RETURN,
                            wScan: 0,
                            dwFlags: KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                SendInput(&[down, up], std::mem::size_of::<INPUT>() as i32);
            }
            "backspace" => {
                let down = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VK_BACK,
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
                            wVk: VK_BACK,
                            wScan: 0,
                            dwFlags: KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                SendInput(&[down, up], std::mem::size_of::<INPUT>() as i32);
            }
            "tab" => {
                let down = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VK_TAB,
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
                            wVk: VK_TAB,
                            wScan: 0,
                            dwFlags: KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                SendInput(&[down, up], std::mem::size_of::<INPUT>() as i32);
            }
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
