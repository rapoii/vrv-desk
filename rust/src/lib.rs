pub mod audio;
pub mod auth;
pub mod dirty_rect;
pub mod discovery;
pub mod file_manager;
pub mod gdi_capture;
pub mod host_service;
pub mod identity;
pub mod mft_encoder;
pub mod monitor;
pub mod pairing;
pub mod platform;
pub mod privacy;
pub mod protocol;
pub mod service_manager;
pub mod stun;
pub mod system_actions;
pub mod transport;
pub mod unattended;
pub mod video;
pub mod virtual_display;

pub use platform::windows_capture::{DxgiCapturer, HybridScreenCapturer};

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}
