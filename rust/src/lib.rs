pub mod discovery;
pub mod identity;
pub mod pairing;
pub mod platform;
pub mod protocol;
pub mod transport;
pub mod gdi_capture;
pub mod auth;

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}
