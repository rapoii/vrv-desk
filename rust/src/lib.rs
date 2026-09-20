pub mod discovery;
pub mod identity;
pub mod pairing;
pub mod platform;
pub mod protocol;
pub mod transport;
pub mod gdi_capture;
pub mod audio;
pub mod video;
pub mod mft_encoder;
pub mod auth;
pub mod stun;
pub mod dirty_rect;
pub mod host_service;
pub mod unattended;
pub mod file_manager;
pub mod service_manager;
pub mod monitor;
pub mod privacy;
pub mod system_actions;
pub mod virtual_display;

pub use platform::windows_capture::{DxgiCapturer, HybridScreenCapturer};

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}
