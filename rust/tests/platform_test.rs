use mirror_core::platform::windows_capture::DxgiCapturer;
use mirror_core::platform::windows_input::inject_input;
use mirror_core::protocol::{InputEvent, MouseButton};

#[test]
fn test_windows_input_injection_does_not_panic() {
    let event = InputEvent::MouseMove { x: 0.5, y: 0.5 };
    let res = inject_input(&event, 1920, 1080);
    assert!(res.is_ok());

    let event_down = InputEvent::MouseDown {
        x: 0.5,
        y: 0.5,
        button: MouseButton::Left,
    };
    let res_down = inject_input(&event_down, 1920, 1080);
    assert!(res_down.is_ok());

    let event_up = InputEvent::MouseUp {
        x: 0.5,
        y: 0.5,
        button: MouseButton::Left,
    };
    let res_up = inject_input(&event_up, 1920, 1080);
    assert!(res_up.is_ok());
}

#[test]
fn test_dxgi_capturer_initialization() {
    let mut capturer = match DxgiCapturer::new() {
        Ok(c) => c,
        Err(e) => {
            println!("DxgiCapturer unavailable in this environment: {}", e);
            return;
        }
    };
    assert!(capturer.width > 0);
    assert!(capturer.height > 0);
    let frame = capturer.acquire_next_frame(100);
    assert!(frame.is_ok());
}
