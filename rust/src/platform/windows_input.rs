use crate::protocol::{InputEvent, MouseButton};
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::*;

#[cfg(windows)]
pub fn inject_input(event: &InputEvent, _screen_w: u32, _screen_h: u32) -> Result<(), String> {
    unsafe {
        match event {
            InputEvent::MouseMove { x, y } => {
                let norm_x = ((x * 65535.0) as i32).clamp(0, 65535);
                let norm_y = ((y * 65535.0) as i32).clamp(0, 65535);
                let input = INPUT {
                    r#type: INPUT_MOUSE,
                    Anonymous: INPUT_0 {
                        mi: MOUSEINPUT {
                            dx: norm_x,
                            dy: norm_y,
                            mouseData: 0,
                            dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
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
                let norm_x = ((x * 65535.0) as i32).clamp(0, 65535);
                let norm_y = ((y * 65535.0) as i32).clamp(0, 65535);
                let move_input = INPUT {
                    r#type: INPUT_MOUSE,
                    Anonymous: INPUT_0 {
                        mi: MOUSEINPUT {
                            dx: norm_x,
                            dy: norm_y,
                            mouseData: 0,
                            dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
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
                SendInput(
                    &[move_input, down_input, up_input],
                    std::mem::size_of::<INPUT>() as i32,
                );
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
