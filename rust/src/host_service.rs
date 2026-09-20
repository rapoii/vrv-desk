use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use crate::audio::AudioLoopbackCapturer;
use crate::auth::AuthGatekeeper;
use crate::discovery::{LanBeacon, LanDiscoveryBroadcaster, DISCOVERY_MULTICAST_ADDR, DISCOVERY_PORT};
use crate::identity::DeviceIdentity;
use crate::platform::windows_capture::HybridScreenCapturer;
use crate::platform::windows_input::{
    get_clipboard_sequence_number, get_clipboard_text, inject_input, inject_shortcut,
    inject_unicode_text, set_clipboard_text,
};
use crate::protocol::{InputEvent, MouseButton};
use crate::stun::{query_stun, DEFAULT_STUN_SERVER};

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum ClientInput {
    #[serde(rename = "mouse_move")]
    MouseMove { x: f32, y: f32 },
    #[serde(rename = "mouse_down")]
    MouseDown { button: Option<String> },
    #[serde(rename = "mouse_up")]
    MouseUp { button: Option<String> },
    #[serde(rename = "touch_tap")]
    TouchTap { x: f32, y: f32 },
    #[serde(rename = "mouse_wheel")]
    MouseWheel { delta_y: f32 },
    #[serde(rename = "key_down")]
    KeyDown { keycode: u32 },
    #[serde(rename = "key_up")]
    KeyUp { keycode: u32 },
    #[serde(rename = "type_text")]
    TypeText { text: String },
    #[serde(rename = "shortcut")]
    Shortcut { name: String },
    #[serde(rename = "clipboard_text")]
    ClipboardText { text: String },
}

#[derive(Clone)]
pub struct HostTelemetry {
    pub is_running: Arc<AtomicBool>,
    pub is_connected: Arc<AtomicBool>,
    pub connected_client: Arc<std::sync::RwLock<Option<String>>>,
    pub fps: Arc<AtomicU32>,
    pub bitrate_kbps: Arc<AtomicU32>,
    pub frames_sent: Arc<AtomicU32>,
    pub frames_skipped: Arc<AtomicU32>,
    pub log_messages: Arc<std::sync::RwLock<Vec<String>>>,
}

impl Default for HostTelemetry {
    fn default() -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(true)),
            is_connected: Arc::new(AtomicBool::new(false)),
            connected_client: Arc::new(std::sync::RwLock::new(None)),
            fps: Arc::new(AtomicU32::new(0)),
            bitrate_kbps: Arc::new(AtomicU32::new(0)),
            frames_sent: Arc::new(AtomicU32::new(0)),
            frames_skipped: Arc::new(AtomicU32::new(0)),
            log_messages: Arc::new(std::sync::RwLock::new(Vec::new())),
        }
    }
}

impl HostTelemetry {
    pub fn add_log(&self, msg: String) {
        if let Ok(mut logs) = self.log_messages.write() {
            if logs.len() >= 50 {
                logs.remove(0);
            }
            logs.push(msg);
        }
    }
}

pub fn get_host_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "Host-PC".to_string())
}

pub fn get_or_generate_pin(cli_pin: Option<String>) -> String {
    if let Some(pin) = cli_pin {
        return pin;
    }
    if let Ok(pin) = std::env::var("VRV_PIN") {
        if !pin.trim().is_empty() {
            return pin.trim().to_string();
        }
    }
    use rand::Rng;
    let pin_num: u32 = rand::thread_rng().gen_range(100_000..=999_999);
    format!("{:06}", pin_num)
}

pub fn get_or_generate_device_id(cli_device_id: Option<String>) -> String {
    if let Some(id) = cli_device_id {
        return id.replace(' ', "");
    }
    if let Ok(id) = std::env::var("VRV_DEVICE_ID") {
        if !id.trim().is_empty() {
            return id.trim().replace(' ', "");
        }
    }
    let identity = DeviceIdentity::generate();
    identity.device_id()
}

pub fn format_six_digit_display(code: &str) -> String {
    let clean = code.replace(' ', "");
    if clean.len() == 6 {
        format!("{} {}", &clean[..3], &clean[3..])
    } else {
        code.to_string()
    }
}

pub async fn run_host_server_loop(
    device_id: String,
    pin: Arc<std::sync::RwLock<String>>,
    host_name: String,
    port: u16,
    signal_url: Option<String>,
    telemetry: HostTelemetry,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let initial_pin = { pin.read().unwrap().clone() };
    telemetry.add_log(format!("Starting VrV Desk Host (Device ID: {})...", format_six_digit_display(&device_id)));

    let stun_endpoint = match query_stun(DEFAULT_STUN_SERVER).await {
        Ok(addr) => Some(addr.to_string()),
        Err(_) => None,
    };
    let stun_display = stun_endpoint
        .as_deref()
        .unwrap_or("Local Network Only");
    telemetry.add_log(format!("STUN NAT resolution: {}", stun_display));

    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
    let listener = TcpListener::bind(&addr).await?;
    telemetry.add_log(format!("Listening on ws://0.0.0.0:{}", port));

    let beacon = LanBeacon::new(device_id.clone(), host_name.clone(), port);
    let _broadcaster = LanDiscoveryBroadcaster::start(beacon, 1500);
    telemetry.add_log(format!("LAN discovery beacon active on UDP :{}", DISCOVERY_PORT));

    if let Some(url) = signal_url {
        let sig_device_id = device_id.clone();
        let sig_host_name = host_name.clone();
        let sig_pin = pin.clone();
        let sig_stun = stun_endpoint.clone();
        let sig_telem = telemetry.clone();
        tokio::spawn(async move {
            run_signal_client(url, sig_device_id, sig_host_name, sig_stun, sig_pin, sig_telem).await;
        });
    }

    while telemetry.is_running.load(Ordering::Relaxed) {
        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                telemetry.add_log(format!("Incoming connection from {}", peer_addr));
                let _ = stream.set_nodelay(true);

                let pin_clone = pin.clone();
                let host_name_clone = host_name.clone();
                let telem_clone = telemetry.clone();

                tokio::spawn(async move {
                    telem_clone.is_connected.store(true, Ordering::Relaxed);
                    *telem_clone.connected_client.write().unwrap() = Some(peer_addr.to_string());

                    let ws_stream = match accept_async(stream).await {
                        Ok(ws) => ws,
                        Err(e) => {
                            telem_clone.add_log(format!("Handshake failed from {}: {:?}", peer_addr, e));
                            telem_clone.is_connected.store(false, Ordering::Relaxed);
                            *telem_clone.connected_client.write().unwrap() = None;
                            return;
                        }
                    };

                    if let Err(e) = handle_streaming_session(
                        ws_stream,
                        peer_addr.to_string(),
                        pin_clone,
                        host_name_clone,
                        telem_clone.clone(),
                    )
                    .await
                    {
                        telem_clone.add_log(format!("Session ended ({}) : {:?}", peer_addr, e));
                    }

                    telem_clone.is_connected.store(false, Ordering::Relaxed);
                    *telem_clone.connected_client.write().unwrap() = None;
                    telem_clone.fps.store(0, Ordering::Relaxed);
                    telem_clone.bitrate_kbps.store(0, Ordering::Relaxed);
                    telem_clone.add_log(format!("Client {} disconnected", peer_addr));
                });
            }
            Err(e) => {
                if !telemetry.is_running.load(Ordering::Relaxed) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    }

    Ok(())
}

async fn run_signal_client(
    signal_url: String,
    device_id: String,
    host_name: String,
    stun_endpoint: Option<String>,
    pin: Arc<std::sync::RwLock<String>>,
    telemetry: HostTelemetry,
) {
    loop {
        if !telemetry.is_running.load(Ordering::Relaxed) {
            break;
        }
        telemetry.add_log(format!("Connecting to signaling server {}...", signal_url));
        match connect_async(&signal_url).await {
            Ok((mut ws_stream, _)) => {
                let reg_msg = serde_json::json!({
                    "type": "register_host",
                    "device_id": device_id,
                    "name": host_name,
                    "stun_endpoint": stun_endpoint
                });

                if ws_stream.send(Message::Text(reg_msg.to_string().into())).await.is_err() {
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    continue;
                }

                let reg_ok = match ws_stream.next().await {
                    Some(Ok(Message::Text(t))) => {
                        let parsed: serde_json::Value = serde_json::from_str(&t).unwrap_or_default();
                        parsed.get("type").and_then(|v| v.as_str()) == Some("register_ok")
                    }
                    _ => false,
                };

                if !reg_ok {
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    continue;
                }

                telemetry.add_log(format!("Registered with signaling broker (ID: {})", device_id));

                let peer_connected = match ws_stream.next().await {
                    Some(Ok(Message::Text(t))) => {
                        let parsed: serde_json::Value = serde_json::from_str(&t).unwrap_or_default();
                        parsed.get("type").and_then(|v| v.as_str()) == Some("client_connected")
                    }
                    _ => false,
                };

                if !peer_connected {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    continue;
                }

                telemetry.add_log("Remote peer connected via signaling broker!".to_string());
                telemetry.is_connected.store(true, Ordering::Relaxed);
                *telemetry.connected_client.write().unwrap() = Some("Remote Peer (Signaled)".to_string());

                let remote_peer_desc = format!("Remote-Peer via Signal ({})", signal_url);
                let _ = handle_streaming_session(
                    ws_stream,
                    remote_peer_desc,
                    pin.clone(),
                    host_name.clone(),
                    telemetry.clone(),
                )
                .await;

                telemetry.is_connected.store(false, Ordering::Relaxed);
                *telemetry.connected_client.write().unwrap() = None;
                telemetry.add_log("Remote session closed. Reconnecting to broker in 2s...".to_string());
            }
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

pub async fn handle_streaming_session<S>(
    ws_stream: tokio_tungstenite::WebSocketStream<S>,
    peer_desc: String,
    pin: Arc<std::sync::RwLock<String>>,
    host_name: String,
    telemetry: HostTelemetry,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    let current_pin = { pin.read().unwrap().clone() };
    let (session_token, e2ee_enabled) = AuthGatekeeper::authenticate_stream(
        &mut ws_sender,
        &mut ws_receiver,
        &current_pin,
        &host_name,
    )
    .await?;

    if e2ee_enabled {
        telemetry.add_log(format!("E2EE Session established with {} (ChaCha20-Poly1305)", peer_desc));
    } else {
        telemetry.add_log(format!("Client authenticated ({})", peer_desc));
    }

    let mut e2ee_frame_session = if e2ee_enabled {
        Some(crate::transport::SecureTransportSession::from_token(&session_token))
    } else {
        None
    };
    let mut e2ee_audio_session = if e2ee_enabled {
        Some(crate::transport::SecureTransportSession::from_token(&session_token))
    } else {
        None
    };
    let mut e2ee_input_session = if e2ee_enabled {
        Some(crate::transport::SecureTransportSession::from_token(&session_token))
    } else {
        None
    };

    let mut capturer = HybridScreenCapturer::new().map_err(|e| format!("Capturer init failed: {}", e))?;
    let screen_w = capturer.screen_width();
    let screen_h = capturer.screen_height();

    let mut h264_encoder = match crate::video::VideoEncoder::new(screen_w, screen_h, 2_500_000, 60.0) {
        Ok(enc) => Some(enc),
        Err(e) => {
            telemetry.add_log(format!("H.264 init fallback: {}", e));
            None
        }
    };

    let ws_sender = Arc::new(tokio::sync::Mutex::new(ws_sender));
    let is_running = Arc::new(AtomicBool::new(true));

    let is_running_frame = is_running.clone();
    let ws_sender_frame = ws_sender.clone();
    let telem_frame = telemetry.clone();

    let frame_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(16));
        let mut sent_in_sec = 0u32;
        let mut bytes_in_sec = 0usize;
        let mut last_metric = tokio::time::Instant::now();

        while is_running_frame.load(Ordering::Relaxed) {
            interval.tick().await;
            let frame_res = if let Some(ref mut enc) = h264_encoder {
                capturer.capture_h264_with_dirty(10, enc).map(|opt| opt.map(|(bytes, _dirty)| bytes))
            } else {
                capturer.capture_jpeg(10, 60, 1024)
            };

            match frame_res {
                Ok(Some(frame_bytes)) => {
                    sent_in_sec += 1;
                    bytes_in_sec += frame_bytes.len();
                    telem_frame.frames_sent.fetch_add(1, Ordering::Relaxed);

                    let out_bytes = if let Some(ref mut sec) = e2ee_frame_session {
                        match sec.encrypt(&frame_bytes) {
                            Ok(enc) => enc,
                            Err(_) => frame_bytes,
                        }
                    } else {
                        frame_bytes
                    };
                    let mut sender = ws_sender_frame.lock().await;
                    if sender.send(Message::Binary(out_bytes.into())).await.is_err() {
                        is_running_frame.store(false, Ordering::Relaxed);
                        break;
                    }
                }
                Ok(None) => {
                    telem_frame.frames_skipped.fetch_add(1, Ordering::Relaxed);
                }
                Err(_) => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }

            if last_metric.elapsed() >= Duration::from_secs(1) {
                telem_frame.fps.store(sent_in_sec, Ordering::Relaxed);
                let kbps = ((bytes_in_sec * 8) / 1000) as u32;
                telem_frame.bitrate_kbps.store(kbps, Ordering::Relaxed);
                sent_in_sec = 0;
                bytes_in_sec = 0;
                last_metric = tokio::time::Instant::now();
            }
        }
    });

    let is_running_clip = is_running.clone();
    let ws_sender_clip = ws_sender.clone();
    let clipboard_task = tokio::spawn(async move {
        let mut last_seq = get_clipboard_sequence_number();
        let mut interval = tokio::time::interval(Duration::from_millis(500));
        while is_running_clip.load(Ordering::Relaxed) {
            interval.tick().await;
            let current_seq = get_clipboard_sequence_number();
            if current_seq != last_seq {
                last_seq = current_seq;
                if let Some(text) = get_clipboard_text() {
                    let sync_msg = serde_json::json!({
                        "type": "clipboard_sync",
                        "text": text
                    });
                    let mut sender = ws_sender_clip.lock().await;
                    if sender.send(Message::Text(sync_msg.to_string().into())).await.is_err() {
                        is_running_clip.store(false, Ordering::Relaxed);
                        break;
                    }
                }
            }
        }
    });

    let is_running_audio = is_running.clone();
    let ws_sender_audio = ws_sender.clone();
    let audio_task = tokio::spawn(async move {
        let mut capturer = AudioLoopbackCapturer::new();
        while is_running_audio.load(Ordering::Relaxed) {
            match capturer.read_packet_timeout(Duration::from_millis(50)) {
                Some(packet) => {
                    let out_bytes = if let Some(ref mut sec) = e2ee_audio_session {
                        match sec.encrypt(&packet) {
                            Ok(enc) => enc,
                            Err(_) => packet,
                        }
                    } else {
                        packet
                    };
                    let mut sender = ws_sender_audio.lock().await;
                    if sender.send(Message::Binary(out_bytes.into())).await.is_err() {
                        is_running_audio.store(false, Ordering::Relaxed);
                        break;
                    }
                }
                None => {
                    tokio::task::yield_now().await;
                }
            }
        }
    });

    let is_running_input = is_running.clone();
    let input_task = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            if !is_running_input.load(Ordering::Relaxed) {
                break;
            }
            let text_opt: Option<String> = match msg {
                Ok(Message::Text(text)) => Some(text.to_string()),
                Ok(Message::Binary(bin_data)) => {
                    if let Some(ref mut sec) = e2ee_input_session {
                        if crate::transport::SecureTransportSession::is_e2ee_packet(&bin_data) {
                            match sec.decrypt(&bin_data) {
                                Ok(plain) => String::from_utf8(plain).ok(),
                                Err(_) => None,
                            }
                        } else {
                            String::from_utf8(bin_data.to_vec()).ok()
                        }
                    } else {
                        String::from_utf8(bin_data.to_vec()).ok()
                    }
                }
                Ok(Message::Close(_)) => {
                    is_running_input.store(false, Ordering::Relaxed);
                    break;
                }
                Err(_) => {
                    is_running_input.store(false, Ordering::Relaxed);
                    break;
                }
                _ => None,
            };

            if let Some(text) = text_opt {
                if let Ok(input) = serde_json::from_str::<ClientInput>(&text) {
                    match input {
                        ClientInput::MouseMove { x, y } => {
                            let _ = inject_input(&InputEvent::MouseMove { x, y }, screen_w as u32, screen_h as u32);
                        }
                        ClientInput::MouseDown { button } => {
                            let btn = match button.as_deref() {
                                Some("right") => MouseButton::Right,
                                Some("middle") => MouseButton::Middle,
                                _ => MouseButton::Left,
                            };
                            let _ = inject_input(&InputEvent::MouseDown { x: 0.0, y: 0.0, button: btn }, screen_w as u32, screen_h as u32);
                        }
                        ClientInput::MouseUp { button } => {
                            let btn = match button.as_deref() {
                                Some("right") => MouseButton::Right,
                                Some("middle") => MouseButton::Middle,
                                _ => MouseButton::Left,
                            };
                            let _ = inject_input(&InputEvent::MouseUp { x: 0.0, y: 0.0, button: btn }, screen_w as u32, screen_h as u32);
                        }
                        ClientInput::TouchTap { x, y } => {
                            let _ = inject_input(&InputEvent::TouchTap { x, y }, screen_w as u32, screen_h as u32);
                        }
                        ClientInput::MouseWheel { delta_y } => {
                            let _ = inject_input(&InputEvent::MouseWheel { delta_y }, screen_w as u32, screen_h as u32);
                        }
                        ClientInput::KeyDown { keycode } => {
                            let _ = inject_input(&InputEvent::KeyDown { keycode }, screen_w as u32, screen_h as u32);
                        }
                        ClientInput::KeyUp { keycode } => {
                            let _ = inject_input(&InputEvent::KeyUp { keycode }, screen_w as u32, screen_h as u32);
                        }
                        ClientInput::TypeText { text } => {
                            let _ = inject_unicode_text(&text);
                        }
                        ClientInput::Shortcut { name } => {
                            let _ = inject_shortcut(&name);
                        }
                        ClientInput::ClipboardText { text } => {
                            set_clipboard_text(&text);
                        }
                    }
                }
            }
        }
        is_running_input.store(false, Ordering::Relaxed);
    });

    let _ = tokio::join!(frame_task, clipboard_task, audio_task, input_task);
    Ok(())
}
