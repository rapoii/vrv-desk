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
use crate::discovery::{
    LanBeacon, LanDiscoveryBroadcaster, DISCOVERY_MULTICAST_ADDR, DISCOVERY_PORT,
};
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
    #[serde(rename = "fs_list")]
    FsList { id: String, path: Option<String> },
    #[serde(rename = "fs_read_chunk")]
    FsReadChunk {
        id: String,
        path: String,
        offset: u64,
        length: Option<usize>,
    },
    #[serde(rename = "fs_write_chunk")]
    FsWriteChunk {
        id: String,
        path: String,
        offset: u64,
        data_b64: String,
        eof: bool,
    },
    #[serde(rename = "fs_mkdir")]
    FsMkdir { id: String, path: String },
    #[serde(rename = "fs_delete")]
    FsDelete {
        id: String,
        path: String,
        is_dir: bool,
    },
    #[serde(rename = "system_sas")]
    SystemSas,
    #[serde(rename = "system_elevate")]
    SystemElevate,
    #[serde(rename = "service_status")]
    ServiceStatus,
    #[serde(rename = "system_desktop_switch")]
    SystemDesktopSwitch,
    #[serde(rename = "get_monitors")]
    GetMonitors,
    #[serde(rename = "switch_monitor")]
    SwitchMonitor { index: u32 },
    #[serde(rename = "set_privacy_mode")]
    SetPrivacyMode { enabled: bool },
    #[serde(rename = "system_action")]
    SystemAction { action: String },
    #[serde(rename = "get_virtual_display_status")]
    GetVirtualDisplayStatus,
    #[serde(rename = "install_virtual_display")]
    InstallVirtualDisplay,
    #[serde(rename = "uninstall_virtual_display")]
    UninstallVirtualDisplay,
    #[serde(rename = "set_quality")]
    SetQuality { profile: String },
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
    unattended: Option<Arc<std::sync::RwLock<crate::unattended::UnattendedConfig>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let unattended_arc = unattended.unwrap_or_else(|| {
        Arc::new(std::sync::RwLock::new(
            crate::unattended::UnattendedConfig::load(),
        ))
    });
    let initial_pin = { pin.read().unwrap().clone() };
    telemetry.add_log(format!(
        "Starting VrV Desk Host (Device ID: {})...",
        format_six_digit_display(&device_id)
    ));
    if unattended_arc.read().unwrap().enabled {
        telemetry.add_log("Unattended Access: ACTIVE (Permanent password set)".to_string());
    } else {
        telemetry.add_log("Unattended Access: Disabled (PIN only)".to_string());
    }

    let stun_endpoint = match query_stun(DEFAULT_STUN_SERVER).await {
        Ok(addr) => Some(addr.to_string()),
        Err(_) => None,
    };
    let stun_display = stun_endpoint.as_deref().unwrap_or("Local Network Only");
    telemetry.add_log(format!("STUN NAT resolution: {}", stun_display));

    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
    let listener = TcpListener::bind(&addr).await?;
    telemetry.add_log(format!("Listening on ws://0.0.0.0:{}", port));

    let beacon = LanBeacon::new(device_id.clone(), host_name.clone(), port);
    let _broadcaster = LanDiscoveryBroadcaster::start(beacon, 1500);
    telemetry.add_log(format!(
        "LAN discovery beacon active on UDP :{}",
        DISCOVERY_PORT
    ));

    if let Some(url) = signal_url {
        let sig_device_id = device_id.clone();
        let sig_host_name = host_name.clone();
        let sig_pin = pin.clone();
        let sig_stun = stun_endpoint.clone();
        let sig_telem = telemetry.clone();
        let sig_unattended = unattended_arc.clone();
        tokio::spawn(async move {
            run_signal_client(
                url,
                sig_device_id,
                sig_host_name,
                sig_stun,
                sig_pin,
                sig_telem,
                sig_unattended,
            )
            .await;
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
                let unattended_clone = unattended_arc.clone();

                tokio::spawn(async move {
                    telem_clone.is_connected.store(true, Ordering::Relaxed);
                    *telem_clone.connected_client.write().unwrap() = Some(peer_addr.to_string());

                    let ws_stream = match accept_async(stream).await {
                        Ok(ws) => ws,
                        Err(e) => {
                            telem_clone
                                .add_log(format!("Handshake failed from {}: {:?}", peer_addr, e));
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
                        Some(unattended_clone),
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
    unattended: Arc<std::sync::RwLock<crate::unattended::UnattendedConfig>>,
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

                if ws_stream
                    .send(Message::Text(reg_msg.to_string().into()))
                    .await
                    .is_err()
                {
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    continue;
                }

                let reg_ok = match ws_stream.next().await {
                    Some(Ok(Message::Text(t))) => {
                        let parsed: serde_json::Value =
                            serde_json::from_str(&t).unwrap_or_default();
                        parsed.get("type").and_then(|v| v.as_str()) == Some("register_ok")
                    }
                    _ => false,
                };

                if !reg_ok {
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    continue;
                }

                telemetry.add_log(format!(
                    "Registered with signaling broker (ID: {})",
                    device_id
                ));

                let peer_connected = match ws_stream.next().await {
                    Some(Ok(Message::Text(t))) => {
                        let parsed: serde_json::Value =
                            serde_json::from_str(&t).unwrap_or_default();
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
                *telemetry.connected_client.write().unwrap() =
                    Some("Remote Peer (Signaled)".to_string());

                let remote_peer_desc = format!("Remote-Peer via Signal ({})", signal_url);
                let _ = handle_streaming_session(
                    ws_stream,
                    remote_peer_desc,
                    pin.clone(),
                    host_name.clone(),
                    telemetry.clone(),
                    Some(unattended.clone()),
                )
                .await;

                telemetry.is_connected.store(false, Ordering::Relaxed);
                *telemetry.connected_client.write().unwrap() = None;
                telemetry
                    .add_log("Remote session closed. Reconnecting to broker in 2s...".to_string());
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
    unattended: Option<Arc<std::sync::RwLock<crate::unattended::UnattendedConfig>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    let current_pin = { pin.read().unwrap().clone() };
    let unattended_snapshot = unattended.as_ref().map(|u| u.read().unwrap().clone());
    let (session_token, e2ee_enabled) = AuthGatekeeper::authenticate_stream_with_unattended(
        &mut ws_sender,
        &mut ws_receiver,
        &current_pin,
        &host_name,
        unattended_snapshot.as_ref(),
    )
    .await?;

    if e2ee_enabled {
        telemetry.add_log(format!(
            "E2EE Session established with {} (ChaCha20-Poly1305)",
            peer_desc
        ));
    } else {
        telemetry.add_log(format!("Client authenticated ({})", peer_desc));
    }

    let mut e2ee_frame_session = if e2ee_enabled {
        Some(crate::transport::SecureTransportSession::from_token(
            &session_token,
        ))
    } else {
        None
    };
    let mut e2ee_audio_session = if e2ee_enabled {
        Some(crate::transport::SecureTransportSession::from_token(
            &session_token,
        ))
    } else {
        None
    };
    let mut e2ee_input_session = if e2ee_enabled {
        Some(crate::transport::SecureTransportSession::from_token(
            &session_token,
        ))
    } else {
        None
    };

    let mut capturer =
        HybridScreenCapturer::new().map_err(|e| format!("Capturer init failed: {}", e))?;
    let screen_w = capturer.screen_width();
    let screen_h = capturer.screen_height();

    let mut h264_encoder =
        match crate::video::VideoEncoder::new(screen_w, screen_h, 2_500_000, 60.0) {
            Ok(enc) => Some(enc),
            Err(e) => {
                telemetry.add_log(format!("H.264 init fallback: {}", e));
                None
            }
        };

    let ws_sender = Arc::new(tokio::sync::Mutex::new(ws_sender));
    let is_running = Arc::new(AtomicBool::new(true));

    let (monitor_tx, mut monitor_rx) = tokio::sync::mpsc::channel::<(
        u32,
        tokio::sync::oneshot::Sender<Result<(u32, u32), String>>,
    )>(4);
    let (quality_tx, mut quality_rx) = tokio::sync::mpsc::channel::<String>(4);

    let is_running_frame = is_running.clone();
    let ws_sender_frame = ws_sender.clone();
    let telem_frame = telemetry.clone();

    let frame_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(16));
        let mut sent_in_sec = 0u32;
        let mut bytes_in_sec = 0usize;
        let mut last_metric = tokio::time::Instant::now();
        let mut current_profile = "balanced".to_string();
        let mut target_w = screen_w;
        let mut target_h = screen_h;
        let mut scale_buffer = Vec::new();

        while is_running_frame.load(Ordering::Relaxed) {
            interval.tick().await;

            // Process dynamic quality profile switch if requested by client
            if let Ok(new_profile) = quality_rx.try_recv() {
                current_profile = new_profile.to_lowercase();
                let cur_screen_w = capturer.screen_width();
                let cur_screen_h = capturer.screen_height();

                let (new_w, new_h, new_bitrate, new_fps, tick_ms) = match current_profile.as_str() {
                    "eco" => {
                        let h = 720u32.min(cur_screen_h) & !1;
                        let mut w = (((cur_screen_w as f32) * (h as f32 / cur_screen_h as f32))
                            as u32)
                            & !1;
                        if w == 0 {
                            w = 1280;
                        }
                        (w, h, 1_200_000, 30.0f32, 33)
                    }
                    "ultra" => (cur_screen_w & !1, cur_screen_h & !1, 6_000_000, 60.0f32, 16),
                    _ => {
                        // balanced (1080p target)
                        let h = 1080u32.min(cur_screen_h) & !1;
                        let mut w = (((cur_screen_w as f32) * (h as f32 / cur_screen_h as f32))
                            as u32)
                            & !1;
                        if w == 0 {
                            w = 1920;
                        }
                        (w, h, 2_500_000, 60.0f32, 16)
                    }
                };

                target_w = new_w;
                target_h = new_h;
                let target_bitrate = new_bitrate;
                let target_fps = new_fps;

                // Re-initialize encoder with target resolution and bitrate
                match crate::video::VideoEncoder::new(
                    target_w,
                    target_h,
                    target_bitrate,
                    target_fps,
                ) {
                    Ok(new_enc) => {
                        h264_encoder = Some(new_enc);
                        telem_frame.add_log(format!(
                            "Stream quality switched to [{}] ({}x{} @ {}fps, {} kbps)",
                            current_profile,
                            target_w,
                            target_h,
                            target_fps as u32,
                            target_bitrate / 1000
                        ));
                    }
                    Err(e) => {
                        telem_frame.add_log(format!(
                            "Encoder re-init failed for {}: {}",
                            current_profile, e
                        ));
                    }
                }
                interval = tokio::time::interval(Duration::from_millis(tick_ms));
            }

            // Process monitor switch request if any
            if let Ok((target_idx, resp_tx)) = monitor_rx.try_recv() {
                let res = capturer.switch_monitor(target_idx);
                if let Ok((w, h)) = res {
                    let monitors = crate::monitor::enumerate_monitors();
                    if let Some(m) = monitors.iter().find(|m| m.index == target_idx) {
                        crate::platform::windows_input::set_active_monitor_bounds(
                            m.left, m.top, m.width, m.height,
                        );
                    }
                    telem_frame.add_log(format!(
                        "Switched monitor capture to Display {} ({}x{})",
                        target_idx + 1,
                        w,
                        h
                    ));
                }
                let _ = resp_tx.send(res);
            }

            let frame_res = if let Some(ref mut enc) = h264_encoder {
                capturer
                    .capture_h264_scaled_with_dirty(10, enc, target_w, target_h, &mut scale_buffer)
                    .map(|opt| opt.map(|(bytes, _dirty)| bytes))
            } else {
                let jpeg_q = match current_profile.as_str() {
                    "eco" => 50,
                    "ultra" => 90,
                    _ => 70,
                };
                capturer.capture_jpeg(10, jpeg_q, target_w)
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
                    if sender
                        .send(Message::Binary(out_bytes.into()))
                        .await
                        .is_err()
                    {
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

    let last_synced_clipboard = Arc::new(tokio::sync::Mutex::new(None::<String>));

    let is_running_clip = is_running.clone();
    let ws_sender_clip = ws_sender.clone();
    let last_synced_clip = last_synced_clipboard.clone();
    let clipboard_task = tokio::spawn(async move {
        let mut last_seq = get_clipboard_sequence_number();
        let mut interval = tokio::time::interval(Duration::from_millis(500));
        while is_running_clip.load(Ordering::Relaxed) {
            interval.tick().await;
            let current_seq = get_clipboard_sequence_number();
            if current_seq != last_seq {
                last_seq = current_seq;
                if let Some(text) = get_clipboard_text() {
                    let mut synced = last_synced_clip.lock().await;
                    if synced.as_ref() == Some(&text) {
                        continue;
                    }
                    *synced = Some(text.clone());
                    drop(synced);

                    let sync_msg = serde_json::json!({
                        "type": "clipboard_sync",
                        "text": text
                    });
                    let mut sender = ws_sender_clip.lock().await;
                    if sender
                        .send(Message::Text(sync_msg.to_string().into()))
                        .await
                        .is_err()
                    {
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
                    if sender
                        .send(Message::Binary(out_bytes.into()))
                        .await
                        .is_err()
                    {
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
    let last_synced_input = last_synced_clipboard.clone();
    let ws_sender_input = ws_sender.clone();
    let telemetry_input = telemetry.clone();
    let monitor_tx_input = monitor_tx.clone();
    let quality_tx_input = quality_tx.clone();
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
                            let _ = inject_input(
                                &InputEvent::MouseMove { x, y },
                                screen_w as u32,
                                screen_h as u32,
                            );
                        }
                        ClientInput::MouseDown { button } => {
                            let btn = match button.as_deref() {
                                Some("right") => MouseButton::Right,
                                Some("middle") => MouseButton::Middle,
                                _ => MouseButton::Left,
                            };
                            let _ = inject_input(
                                &InputEvent::MouseDown {
                                    x: 0.0,
                                    y: 0.0,
                                    button: btn,
                                },
                                screen_w as u32,
                                screen_h as u32,
                            );
                        }
                        ClientInput::MouseUp { button } => {
                            let btn = match button.as_deref() {
                                Some("right") => MouseButton::Right,
                                Some("middle") => MouseButton::Middle,
                                _ => MouseButton::Left,
                            };
                            let _ = inject_input(
                                &InputEvent::MouseUp {
                                    x: 0.0,
                                    y: 0.0,
                                    button: btn,
                                },
                                screen_w as u32,
                                screen_h as u32,
                            );
                        }
                        ClientInput::TouchTap { x, y } => {
                            let _ = inject_input(
                                &InputEvent::TouchTap { x, y },
                                screen_w as u32,
                                screen_h as u32,
                            );
                        }
                        ClientInput::MouseWheel { delta_y } => {
                            let _ = inject_input(
                                &InputEvent::MouseWheel { delta_y },
                                screen_w as u32,
                                screen_h as u32,
                            );
                        }
                        ClientInput::KeyDown { keycode } => {
                            let _ = inject_input(
                                &InputEvent::KeyDown { keycode },
                                screen_w as u32,
                                screen_h as u32,
                            );
                        }
                        ClientInput::KeyUp { keycode } => {
                            let _ = inject_input(
                                &InputEvent::KeyUp { keycode },
                                screen_w as u32,
                                screen_h as u32,
                            );
                        }
                        ClientInput::TypeText { text } => {
                            let _ = inject_unicode_text(&text);
                        }
                        ClientInput::Shortcut { name } => {
                            let _ = inject_shortcut(&name);
                        }
                        ClientInput::ClipboardText { text } => {
                            {
                                let mut synced = last_synced_input.lock().await;
                                *synced = Some(text.clone());
                            }
                            set_clipboard_text(&text);
                        }
                        ClientInput::FsList { id, path } => {
                            let p = path.unwrap_or_default();
                            let resp = match crate::file_manager::FileManager::list_directory(&p) {
                                Ok((canonical_path, entries)) => {
                                    serde_json::to_string(&crate::file_manager::FsListResponse {
                                        msg_type: "fs_list_resp".to_string(),
                                        id,
                                        path: canonical_path,
                                        entries,
                                    })
                                    .unwrap()
                                }
                                Err(e) => {
                                    serde_json::to_string(&crate::file_manager::FsErrorResponse {
                                        msg_type: "fs_error".to_string(),
                                        id,
                                        error: e,
                                    })
                                    .unwrap()
                                }
                            };
                            let mut sender = ws_sender_input.lock().await;
                            let _ = sender.send(Message::Text(resp.into())).await;
                        }
                        ClientInput::FsReadChunk {
                            id,
                            path,
                            offset,
                            length,
                        } => {
                            let len = length.unwrap_or(65536);
                            let resp = match crate::file_manager::FileManager::read_chunk(
                                &path, offset, len,
                            ) {
                                Ok(mut r) => {
                                    r.id = id;
                                    serde_json::to_string(&r).unwrap()
                                }
                                Err(e) => {
                                    serde_json::to_string(&crate::file_manager::FsErrorResponse {
                                        msg_type: "fs_error".to_string(),
                                        id,
                                        error: e,
                                    })
                                    .unwrap()
                                }
                            };
                            let mut sender = ws_sender_input.lock().await;
                            let _ = sender.send(Message::Text(resp.into())).await;
                        }
                        ClientInput::FsWriteChunk {
                            id,
                            path,
                            offset,
                            data_b64,
                            eof,
                        } => {
                            let resp = match crate::file_manager::FileManager::write_chunk(
                                &path, offset, &data_b64, eof,
                            ) {
                                Ok(mut r) => {
                                    r.id = id;
                                    serde_json::to_string(&r).unwrap()
                                }
                                Err(e) => {
                                    serde_json::to_string(&crate::file_manager::FsErrorResponse {
                                        msg_type: "fs_error".to_string(),
                                        id,
                                        error: e,
                                    })
                                    .unwrap()
                                }
                            };
                            let mut sender = ws_sender_input.lock().await;
                            let _ = sender.send(Message::Text(resp.into())).await;
                        }
                        ClientInput::FsMkdir { id, path } => {
                            let resp = match crate::file_manager::FileManager::create_dir(&path) {
                                Ok(_) => {
                                    telemetry_input
                                        .add_log(format!("Remote created dir: {}", path));
                                    serde_json::to_string(&crate::file_manager::FsActionResponse {
                                        msg_type: "fs_action_resp".to_string(),
                                        id,
                                        action: "mkdir".to_string(),
                                        success: true,
                                    })
                                    .unwrap()
                                }
                                Err(e) => {
                                    serde_json::to_string(&crate::file_manager::FsErrorResponse {
                                        msg_type: "fs_error".to_string(),
                                        id,
                                        error: e,
                                    })
                                    .unwrap()
                                }
                            };
                            let mut sender = ws_sender_input.lock().await;
                            let _ = sender.send(Message::Text(resp.into())).await;
                        }
                        ClientInput::FsDelete { id, path, is_dir } => {
                            let resp = match crate::file_manager::FileManager::delete_item(
                                &path, is_dir,
                            ) {
                                Ok(_) => {
                                    telemetry_input.add_log(format!("Remote deleted: {}", path));
                                    serde_json::to_string(&crate::file_manager::FsActionResponse {
                                        msg_type: "fs_action_resp".to_string(),
                                        id,
                                        action: "delete".to_string(),
                                        success: true,
                                    })
                                    .unwrap()
                                }
                                Err(e) => {
                                    serde_json::to_string(&crate::file_manager::FsErrorResponse {
                                        msg_type: "fs_error".to_string(),
                                        id,
                                        error: e,
                                    })
                                    .unwrap()
                                }
                            };
                            let mut sender = ws_sender_input.lock().await;
                            let _ = sender.send(Message::Text(resp.into())).await;
                        }
                        ClientInput::SystemSas => {
                            telemetry_input.add_log(
                                "Client requested Secure Attention Sequence (Ctrl+Alt+Del)"
                                    .to_string(),
                            );
                            let pipe_res =
                                crate::service_manager::send_pipe_command(r#"{"cmd":"sas"}"#).await;
                            let success = if pipe_res.is_ok() {
                                true
                            } else {
                                crate::service_manager::trigger_sas().is_ok()
                            };
                            let resp = serde_json::json!({
                                "type": "system_sas_result",
                                "success": success,
                            })
                            .to_string();
                            let mut sender = ws_sender_input.lock().await;
                            let _ = sender.send(Message::Text(resp.into())).await;
                        }
                        ClientInput::SystemElevate => {
                            telemetry_input
                                .add_log("Client requested host UAC elevation".to_string());
                            let curr_exe = std::env::current_exe().unwrap_or_default();
                            let exe_str = curr_exe.to_str().unwrap_or("");
                            let res = crate::service_manager::request_elevation(
                                Some(exe_str),
                                Some("--elevated"),
                            );
                            let resp = serde_json::json!({
                                "type": "system_elevate_result",
                                "success": res.is_ok(),
                                "error": res.err(),
                            })
                            .to_string();
                            let mut sender = ws_sender_input.lock().await;
                            let _ = sender.send(Message::Text(resp.into())).await;
                        }
                        ClientInput::ServiceStatus => {
                            let status = crate::service_manager::get_service_status();
                            let resp = serde_json::json!({
                                "type": "service_status_result",
                                "installed": status.installed,
                                "running": status.running,
                                "elevated": status.elevated,
                                "is_service": status.is_service,
                            })
                            .to_string();
                            let mut sender = ws_sender_input.lock().await;
                            let _ = sender.send(Message::Text(resp.into())).await;
                        }
                        ClientInput::SystemDesktopSwitch => {
                            telemetry_input.add_log(
                                "Client requested desktop switch to active input desktop"
                                    .to_string(),
                            );
                            let res = crate::service_manager::switch_to_input_desktop();
                            let resp = serde_json::json!({
                                "type": "system_desktop_switch_result",
                                "success": res.is_ok(),
                                "error": res.err(),
                            })
                            .to_string();
                            let mut sender = ws_sender_input.lock().await;
                            let _ = sender.send(Message::Text(resp.into())).await;
                        }
                        ClientInput::GetMonitors => {
                            let monitors = crate::monitor::enumerate_monitors();
                            let resp = serde_json::json!({
                                "type": "monitors_list",
                                "monitors": monitors,
                            })
                            .to_string();
                            let mut sender = ws_sender_input.lock().await;
                            let _ = sender.send(Message::Text(resp.into())).await;
                        }
                        ClientInput::SwitchMonitor { index } => {
                            telemetry_input
                                .add_log(format!("Client requested switch to monitor {}", index));
                            let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                            let send_ok = monitor_tx_input.send((index, resp_tx)).await.is_ok();
                            let res = if send_ok {
                                resp_rx
                                    .await
                                    .unwrap_or(Err("Capture thread closed".to_string()))
                            } else {
                                Err("Failed to queue monitor switch".to_string())
                            };
                            let resp = match res {
                                Ok((w, h)) => serde_json::json!({
                                    "type": "switch_monitor_res",
                                    "success": true,
                                    "index": index,
                                    "width": w,
                                    "height": h,
                                    "error": null,
                                }),
                                Err(e) => serde_json::json!({
                                    "type": "switch_monitor_res",
                                    "success": false,
                                    "index": index,
                                    "error": e,
                                }),
                            }
                            .to_string();
                            let mut sender = ws_sender_input.lock().await;
                            let _ = sender.send(Message::Text(resp.into())).await;
                        }
                        ClientInput::SetPrivacyMode { enabled } => {
                            telemetry_input.add_log(format!("Setting privacy mode: {}", enabled));
                            let res = crate::privacy::set_privacy_mode(enabled);
                            let resp = serde_json::json!({
                                "type": "privacy_mode_res",
                                "enabled": enabled,
                                "success": res.is_ok(),
                                "error": res.err(),
                            })
                            .to_string();
                            let mut sender = ws_sender_input.lock().await;
                            let _ = sender.send(Message::Text(resp.into())).await;
                        }
                        ClientInput::SystemAction { action } => {
                            telemetry_input
                                .add_log(format!("Executing remote system action: {}", action));
                            let res = crate::system_actions::execute_system_action(&action);
                            let (success, msg) = match res {
                                Ok(m) => (true, m),
                                Err(e) => (false, e),
                            };
                            let resp = serde_json::json!({
                                "type": "system_action_res",
                                "action": action,
                                "success": success,
                                "message": msg,
                            })
                            .to_string();
                            let mut sender = ws_sender_input.lock().await;
                            let _ = sender.send(Message::Text(resp.into())).await;
                        }
                        ClientInput::GetVirtualDisplayStatus => {
                            let status = crate::virtual_display::get_status();
                            let resp = serde_json::json!({
                                "type": "virtual_display_status_res",
                                "driver_installed": status.driver_installed,
                                "driver_name": status.driver_name,
                                "active": status.active,
                                "active_count": status.active_count,
                                "modes": status.modes,
                                "is_headless": status.is_headless,
                                "physical_monitor_count": status.physical_monitor_count,
                            })
                            .to_string();
                            let mut sender = ws_sender_input.lock().await;
                            let _ = sender.send(Message::Text(resp.into())).await;
                        }
                        ClientInput::InstallVirtualDisplay => {
                            telemetry_input.add_log(
                                "Client requested virtual display driver installation".to_string(),
                            );
                            let result = match crate::service_manager::send_pipe_command(
                                r#"{"cmd":"install_virtual_display"}"#,
                            )
                            .await
                            {
                                Ok(pipe_str) => {
                                    if let Ok(pipe_resp) = serde_json::from_str::<
                                        crate::service_manager::PipeResponse,
                                    >(
                                        &pipe_str
                                    ) {
                                        if pipe_resp.status == "ok" {
                                            Ok(pipe_resp.message.unwrap_or_else(|| {
                                                "Driver installed via service".to_string()
                                            }))
                                        } else {
                                            crate::virtual_display::install_driver(None)
                                        }
                                    } else {
                                        crate::virtual_display::install_driver(None)
                                    }
                                }
                                Err(_) => crate::virtual_display::install_driver(None),
                            };
                            let (success, msg) = match result {
                                Ok(m) => (true, m),
                                Err(e) => (false, e),
                            };
                            let resp = serde_json::json!({
                                "type": "install_virtual_display_res",
                                "success": success,
                                "message": msg,
                            })
                            .to_string();
                            let mut sender = ws_sender_input.lock().await;
                            let _ = sender.send(Message::Text(resp.into())).await;
                        }
                        ClientInput::UninstallVirtualDisplay => {
                            telemetry_input.add_log(
                                "Client requested virtual display driver uninstallation"
                                    .to_string(),
                            );
                            let result = match crate::service_manager::send_pipe_command(
                                r#"{"cmd":"uninstall_virtual_display"}"#,
                            )
                            .await
                            {
                                Ok(pipe_str) => {
                                    if let Ok(pipe_resp) = serde_json::from_str::<
                                        crate::service_manager::PipeResponse,
                                    >(
                                        &pipe_str
                                    ) {
                                        if pipe_resp.status == "ok" {
                                            Ok(pipe_resp.message.unwrap_or_else(|| {
                                                "Driver uninstalled via service".to_string()
                                            }))
                                        } else {
                                            crate::virtual_display::uninstall_driver()
                                        }
                                    } else {
                                        crate::virtual_display::uninstall_driver()
                                    }
                                }
                                Err(_) => crate::virtual_display::uninstall_driver(),
                            };
                            let (success, msg) = match result {
                                Ok(m) => (true, m),
                                Err(e) => (false, e),
                            };
                            let resp = serde_json::json!({
                                "type": "uninstall_virtual_display_res",
                                "success": success,
                                "message": msg,
                            })
                            .to_string();
                            let mut sender = ws_sender_input.lock().await;
                            let _ = sender.send(Message::Text(resp.into())).await;
                        }
                        ClientInput::SetQuality { profile } => {
                            telemetry_input.add_log(format!(
                                "Client requested streaming quality profile: {}",
                                profile
                            ));
                            let _ = quality_tx_input.send(profile.clone()).await;

                            let (w, h, kbps, fps) = match profile.to_lowercase().as_str() {
                                "eco" => {
                                    let h = 720u32.min(screen_h as u32) & !1;
                                    let mut w = (((screen_w as f32) * (h as f32 / screen_h as f32))
                                        as u32)
                                        & !1;
                                    if w == 0 {
                                        w = 1280;
                                    }
                                    (w, h, 1200, 30)
                                }
                                "ultra" => (screen_w as u32 & !1, screen_h as u32 & !1, 6000, 60),
                                _ => {
                                    let h = 1080u32.min(screen_h as u32) & !1;
                                    let mut w = (((screen_w as f32) * (h as f32 / screen_h as f32))
                                        as u32)
                                        & !1;
                                    if w == 0 {
                                        w = 1920;
                                    }
                                    (w, h, 2500, 60)
                                }
                            };

                            let resp = serde_json::json!({
                                "type": "quality_changed",
                                "profile": profile,
                                "width": w,
                                "height": h,
                                "bitrate_kbps": kbps,
                                "fps": fps,
                            })
                            .to_string();
                            let mut sender = ws_sender_input.lock().await;
                            let _ = sender.send(Message::Text(resp.into())).await;
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
