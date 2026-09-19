use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum InputEvent {
    MouseMove {
        x: f32,
        y: f32,
    },
    MouseDown {
        x: f32,
        y: f32,
        button: MouseButton,
    },
    MouseUp {
        x: f32,
        y: f32,
        button: MouseButton,
    },
    MouseWheel {
        delta_y: f32,
    },
    KeyDown {
        keycode: u32,
    },
    KeyUp {
        keycode: u32,
    },
    TouchTap {
        x: f32,
        y: f32,
    },
    TouchSwipe {
        start_x: f32,
        start_y: f32,
        end_x: f32,
        end_y: f32,
        duration_ms: u32,
    },
    SystemKey {
        action: String,
    }, // "back", "home", "recents"
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum WirePacket {
    Video {
        timestamp_us: u64,
        is_keyframe: bool,
        payload: Vec<u8>,
    },
    Audio {
        timestamp_us: u64,
        payload: Vec<u8>,
    },
    Input(InputEvent),
    Ping {
        send_ts: u64,
    },
    Pong {
        send_ts: u64,
        echo_ts: u64,
    },
}

impl WirePacket {
    pub fn serialize(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}
