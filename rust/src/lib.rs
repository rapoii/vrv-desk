pub mod discovery;
pub mod identity;
pub mod pairing;
pub mod platform;
pub mod protocol;
pub mod transport;
pub mod gdi_capture;
pub mod audio;
pub mod auth;
pub mod stun;

pub use platform::windows_capture::{DxgiCapturer, HybridScreenCapturer};

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}
