//! Privacy Mode: Screen Blanking and Physical Input Blocking
//! Prevents local physical onlookers from observing screen activity or interfering with inputs.

use std::sync::atomic::{AtomicBool, Ordering};

static PRIVACY_MODE_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn is_privacy_mode_enabled() -> bool {
    PRIVACY_MODE_ACTIVE.load(Ordering::SeqCst)
}

pub fn set_privacy_mode(enable: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::{BOOL, LPARAM, WPARAM};
        use windows::Win32::UI::Input::KeyboardAndMouse::BlockInput;
        use windows::Win32::UI::WindowsAndMessaging::{HWND_BROADCAST, WM_SYSCOMMAND};

        const SC_MONITORPOWER: usize = 0xF170;

        unsafe {
            // Block or unblock physical input (mouse & keyboard)
            let _ = BlockInput(BOOL::from(enable));

            // Blank or wake physical monitors (asynchronously without blocking)
            let lparam_val: isize = if enable { 2 } else { -1 };
            let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                HWND_BROADCAST,
                WM_SYSCOMMAND,
                WPARAM(SC_MONITORPOWER),
                LPARAM(lparam_val),
            );
        }

        PRIVACY_MODE_ACTIVE.store(enable, Ordering::SeqCst);
        Ok(())
    }
    #[cfg(not(windows))]
    {
        PRIVACY_MODE_ACTIVE.store(enable, Ordering::SeqCst);
        Ok(())
    }
}
