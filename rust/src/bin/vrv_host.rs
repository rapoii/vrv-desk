use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use mirror_core::auth::AuthGatekeeper;
use mirror_core::audio::AudioLoopbackCapturer;
use mirror_core::discovery::{
    LanBeacon, LanDiscoveryBroadcaster, DISCOVERY_MULTICAST_ADDR, DISCOVERY_PORT,
};
use mirror_core::platform::windows_capture::HybridScreenCapturer;
use mirror_core::identity::DeviceIdentity;
use mirror_core::platform::windows_input::{
    get_clipboard_sequence_number, get_clipboard_text, inject_input, inject_shortcut,
    inject_unicode_text, set_clipboard_text,
};
use mirror_core::protocol::{InputEvent, MouseButton};
use mirror_core::stun::{query_stun, DEFAULT_STUN_SERVER};

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
    FsReadChunk { id: String, path: String, offset: u64, length: Option<usize> },
    #[serde(rename = "fs_write_chunk")]
    FsWriteChunk { id: String, path: String, offset: u64, data_b64: String, eof: bool },
    #[serde(rename = "fs_mkdir")]
    FsMkdir { id: String, path: String },
    #[serde(rename = "fs_delete")]
    FsDelete { id: String, path: String, is_dir: bool },
    #[serde(rename = "system_sas")]
    SystemSas,
    #[serde(rename = "system_elevate")]
    SystemElevate,
    #[serde(rename = "service_status")]
    ServiceStatus,
    #[serde(rename = "system_desktop_switch")]
    SystemDesktopSwitch,
}

fn parse_cli_args() -> (Option<String>, Option<String>, Option<String>) {
    let mut pin = None;
    let mut device_id = None;
    let mut signal_url = None;

    let args: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--pin" => {
                if i + 1 < args.len() {
                    pin = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--device-id" => {
                if i + 1 < args.len() {
                    device_id = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--signal" => {
                if i + 1 < args.len() {
                    signal_url = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    (pin, device_id, signal_url)
}

fn get_or_generate_pin(cli_pin: Option<String>) -> String {
    if let Some(pin) = cli_pin {
        return pin;
    }
    if let Ok(pin) = env::var("VRV_PIN") {
        if !pin.trim().is_empty() {
            return pin.trim().to_string();
        }
    }
    use rand::Rng;
    let pin_num: u32 = rand::thread_rng().gen_range(100_000..=999_999);
    format!("{:06}", pin_num)
}

fn get_or_generate_device_id(cli_device_id: Option<String>) -> String {
    if let Some(id) = cli_device_id {
        return id.replace(' ', "");
    }
    if let Ok(id) = env::var("VRV_DEVICE_ID") {
        if !id.trim().is_empty() {
            return id.trim().replace(' ', "");
        }
    }
    let identity = DeviceIdentity::generate();
    identity.device_id()
}

fn format_six_digit_display(code: &str) -> String {
    let clean = code.replace(' ', "");
    if clean.len() == 6 {
        format!("{} {}", &clean[..3], &clean[3..])
    } else {
        code.to_string()
    }
}

fn get_host_name() -> String {
    env::var("COMPUTERNAME")
        .or_else(|_| env::var("HOSTNAME"))
        .unwrap_or_else(|_| "Host-PC".to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (cli_pin, cli_device_id, cli_signal_url) = parse_cli_args();
    let pin = Arc::new(get_or_generate_pin(cli_pin));
    let device_id = get_or_generate_device_id(cli_device_id);
    let host_name = get_host_name();

    // Query STUN endpoint (gracefully fallback if offline)
    let stun_endpoint = match query_stun(DEFAULT_STUN_SERVER).await {
        Ok(addr) => Some(addr.to_string()),
        Err(_) => None,
    };
    let stun_display = stun_endpoint
        .as_deref()
        .unwrap_or("Unavailable / Local Only");

    let signal_url = cli_signal_url
        .or_else(|| env::var("VRV_SIGNAL_URL").ok())
        .unwrap_or_else(|| "ws://127.0.0.1:53212".to_string());

    let addr: SocketAddr = "0.0.0.0:53211".parse()?;
    let listener = TcpListener::bind(&addr).await?;

    println!("=================================================");
    println!("🌐 VrV Desk Remote Host Ready!");
    println!("📱 Device ID:   {}", format_six_digit_display(&device_id));
    println!("🔐 Session PIN: {}", format_six_digit_display(&pin));
    let unattended_cfg = mirror_core::unattended::UnattendedConfig::load();
    if unattended_cfg.enabled {
        println!("🔑 Unattended:  ENABLED (Permanent password active)");
    } else {
        println!("🔑 Unattended:  Disabled");
    }
    println!("📡 STUN Public: {}", stun_display);
    println!("📶 Local LAN:   ws://{}", addr);
    println!("=================================================");

    // Start zero-config LAN discovery UDP broadcaster
    let beacon = LanBeacon::new(device_id.clone(), host_name.clone(), 53211);
    let _broadcaster = LanDiscoveryBroadcaster::start(beacon, 1500);
    println!("📡 LAN discovery broadcaster started on UDP port {} (multicast: {})", DISCOVERY_PORT, DISCOVERY_MULTICAST_ADDR);

    let screen_w = unsafe {
        windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
            windows::Win32::UI::WindowsAndMessaging::SM_CXSCREEN,
        )
    };
    let screen_h = unsafe {
        windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
            windows::Win32::UI::WindowsAndMessaging::SM_CYSCREEN,
        )
    };
    println!("🖥️ Primary Screen detected: {}x{}", screen_w, screen_h);

    // Spawn background task for Remote Signaling Broker Registration & Bridging
    let signal_pin = pin.clone();
    let signal_device_id = device_id.clone();
    let signal_host_name = host_name.clone();
    let signal_stun = stun_endpoint.clone();
    tokio::spawn(async move {
        run_signal_client(
            signal_url,
            signal_device_id,
            signal_host_name,
            signal_stun,
            signal_pin,
        )
        .await;
    });

    // Accept local direct LAN connections
    while let Ok((stream, peer_addr)) = listener.accept().await {
        println!("🔗 New local client connected from: {}", peer_addr);
        let _ = stream.set_nodelay(true);

        let pin_clone = pin.clone();
        let host_name_clone = host_name.clone();
        tokio::spawn(async move {
            let ws_stream = match accept_async(stream).await {
                Ok(ws) => ws,
                Err(e) => {
                    eprintln!("Local WebSocket handshake failed: {:?}", e);
                    return;
                }
            };
            if let Err(e) = handle_streaming_session(ws_stream, peer_addr.to_string(), pin_clone, host_name_clone).await {
                eprintln!("Session error with {}: {:?}", peer_addr, e);
            }
            println!("🔌 Local client {} disconnected.", peer_addr);
        });
    }

    Ok(())
}

/// Maintain persistent signaling connection to broker and serve remote client sessions
async fn run_signal_client(
    signal_url: String,
    device_id: String,
    host_name: String,
    stun_endpoint: Option<String>,
    pin: Arc<String>,
) {
    loop {
        println!("[SignalClient] Connecting to signaling broker at {}...", signal_url);
        match connect_async(&signal_url).await {
            Ok((mut ws_stream, _)) => {
                println!("[SignalClient] Connected! Registering device ID: {}", device_id);

                let reg_msg = serde_json::json!({
                    "type": "register_host",
                    "device_id": device_id,
                    "name": host_name,
                    "stun_endpoint": stun_endpoint
                });

                if let Err(e) = ws_stream.send(Message::Text(reg_msg.to_string().into())).await {
                    eprintln!("[SignalClient] Registration send failed: {:?}", e);
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    continue;
                }

                // Wait for register_ok confirmation
                let reg_ok = match ws_stream.next().await {
                    Some(Ok(Message::Text(t))) => {
                        let parsed: serde_json::Value = serde_json::from_str(&t).unwrap_or_default();
                        parsed.get("type").and_then(|v| v.as_str()) == Some("register_ok")
                    }
                    _ => false,
                };

                if !reg_ok {
                    eprintln!("[SignalClient] Failed to receive register_ok from signaling broker");
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    continue;
                }

                println!("✅ [SignalClient] Registered successfully with broker as Device ID: {}", device_id);

                // Wait for the broker to bridge an incoming client connection.
                // The broker sends a rendezvous notification: {"type": "client_connected"}
                let peer_connected = match ws_stream.next().await {
                    Some(Ok(Message::Text(t))) => {
                        let parsed: serde_json::Value = serde_json::from_str(&t).unwrap_or_default();
                        parsed.get("type").and_then(|v| v.as_str()) == Some("client_connected")
                    }
                    _ => false,
                };

                if !peer_connected {
                    eprintln!("[SignalClient] Connection closed before client connected");
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    continue;
                }

                println!("🚀 [SignalClient] Remote client connected! Initiating streaming session...");

                // When a remote client connects, the signaling broker bridges this WebSocket connection directly!
                // We now execute the standard streaming session (Auth -> JPEG frames & Inputs) over ws_stream.
                let remote_peer_desc = format!("Remote-Peer via Signal ({})", signal_url);
                if let Err(e) = handle_streaming_session(
                    ws_stream,
                    remote_peer_desc,
                    pin.clone(),
                    host_name.clone(),
                )
                .await
                {
                    eprintln!("[SignalClient] Remote session ended or failed: {:?}", e);
                }

                println!("[SignalClient] Remote session closed. Re-registering with broker in 2s...");
            }
            Err(e) => {
                eprintln!("[SignalClient] Cannot connect to signaling broker: {:?}. Retrying in 5s...", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

/// Generic streaming session handler that works over both direct LAN and signaled Remote connections
pub async fn handle_streaming_session<S>(
    ws_stream: tokio_tungstenite::WebSocketStream<S>,
    peer_desc: String,
    pin: Arc<String>,
    host_name: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Authenticate client with PIN gatekeeper or Unattended permanent password
    let unattended_cfg = mirror_core::unattended::UnattendedConfig::load();
    let (session_token, e2ee_enabled) = AuthGatekeeper::authenticate_stream_with_unattended(
        &mut ws_sender,
        &mut ws_receiver,
        &pin,
        &host_name,
        Some(&unattended_cfg),
    )
    .await?;

    if e2ee_enabled {
        println!("🔒 E2EE Session established with peer {} (ChaCha20-Poly1305 AEAD active)", peer_desc);
    } else {
        println!("✅ Peer {} authenticated successfully (Standard stream)", peer_desc);
    }

    let mut e2ee_frame_session = if e2ee_enabled {
        Some(mirror_core::transport::SecureTransportSession::from_token(&session_token))
    } else {
        None
    };
    let mut e2ee_audio_session = if e2ee_enabled {
        Some(mirror_core::transport::SecureTransportSession::from_token(&session_token))
    } else {
        None
    };
    let mut e2ee_input_session = if e2ee_enabled {
        Some(mirror_core::transport::SecureTransportSession::from_token(&session_token))
    } else {
        None
    };

    // Initialize capturer only after successful authentication
    let mut capturer = HybridScreenCapturer::new().map_err(|e| format!("Capturer init failed: {}", e))?;
    if capturer.is_dxgi() {
        println!("🚀 Screen capture engine: DirectX 11 DXGI (Hardware GPU Acceleration)");
    } else {
        println!("⚠️ Screen capture engine: GDI BitBlt (Software CPU Fallback)");
    }
    let screen_w = capturer.screen_width();
    let screen_h = capturer.screen_height();

    let mut h264_encoder = match mirror_core::video::VideoEncoder::new(screen_w, screen_h, 2_500_000, 60.0) {
        Ok(enc) => Some(enc),
        Err(e) => {
            println!("⚠️ Video encoder init failed ({}), falling back to JPEG frames", e);
            None
        }
    };

    let ws_sender = std::sync::Arc::new(tokio::sync::Mutex::new(ws_sender));
    let is_running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));

    let is_running_frame = is_running.clone();
    let ws_sender_frame = ws_sender.clone();
    // 1. Task: Stream H.264 / JPEG frames at target ~60 FPS with smart delta capture
    let frame_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(16));
        let mut sent_count = 0u64;
        let mut skipped_count = 0u64;
        let mut last_log = tokio::time::Instant::now();

        while is_running_frame.load(std::sync::atomic::Ordering::Relaxed) {
            interval.tick().await;
            let frame_res = if let Some(ref mut enc) = h264_encoder {
                capturer.capture_h264_with_dirty(10, enc).map(|opt| opt.map(|(bytes, _dirty)| bytes))
            } else {
                capturer.capture_jpeg(10, 60, 1024)
            };

            match frame_res {
                Ok(Some(frame_bytes)) => {
                    sent_count += 1;
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
                        is_running_frame.store(false, std::sync::atomic::Ordering::Relaxed);
                        break;
                    }
                }
                Ok(None) => {
                    // Frame unchanged (static screen), skip send to conserve CPU & bandwidth
                    skipped_count += 1;
                }
                Err(e) => {
                    eprintln!("Capture error: {}", e);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }

            if last_log.elapsed() >= Duration::from_secs(5) {
                let total = sent_count + skipped_count;
                if total > 0 {
                    let savings = (skipped_count as f64 / total as f64) * 100.0;
                    println!(
                        "📊 Video Stream: {} sent, {} skipped static frames (Dirty Region savings: {:.1}%)",
                        sent_count, skipped_count, savings
                    );
                }
                last_log = tokio::time::Instant::now();
            }
        }
    });

    // 2. Task: Clipboard monitor loop
    let last_synced_clipboard = Arc::new(tokio::sync::Mutex::new(None::<String>));

    let is_running_clip = is_running.clone();
    let ws_sender_clip = ws_sender.clone();
    let last_synced_clip = last_synced_clipboard.clone();
    let clipboard_task = tokio::spawn(async move {
        let mut last_seq = get_clipboard_sequence_number();
        let mut interval = tokio::time::interval(Duration::from_millis(500));
        while is_running_clip.load(std::sync::atomic::Ordering::Relaxed) {
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
                        is_running_clip.store(false, std::sync::atomic::Ordering::Relaxed);
                        break;
                    }
                }
            }
        }
    });

    // 3. Task: Audio loopback streaming
    let is_running_audio = is_running.clone();
    let ws_sender_audio = ws_sender.clone();
    let audio_task = tokio::spawn(async move {
        let mut capturer = AudioLoopbackCapturer::new();
        println!(
            "🔊 Audio capturer started (format: 0x{:02x}, channels: {}, rate: {}, mock: {})",
            capturer.format, capturer.channels, capturer.sample_rate, capturer.is_mock
        );

        while is_running_audio.load(std::sync::atomic::Ordering::Relaxed) {
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
                        is_running_audio.store(false, std::sync::atomic::Ordering::Relaxed);
                        break;
                    }
                }
                None => {
                    tokio::task::yield_now().await;
                }
            }
        }
    });

    // 3. Task: Receive and inject input events
    let is_running_input = is_running.clone();
    let last_synced_input = last_synced_clipboard.clone();
    let ws_sender_input = ws_sender.clone();
    let input_task = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            if !is_running_input.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            let text_opt: Option<String> = match msg {
                Ok(Message::Text(text)) => Some(text.to_string()),
                Ok(Message::Binary(bin_data)) => {
                    if let Some(ref mut sec) = e2ee_input_session {
                        if mirror_core::transport::SecureTransportSession::is_e2ee_packet(&bin_data) {
                            match sec.decrypt(&bin_data) {
                                Ok(plain) => String::from_utf8(plain).ok(),
                                Err(e) => {
                                    eprintln!("⚠️ E2EE input decryption failed: {}", e);
                                    None
                                }
                            }
                        } else {
                            String::from_utf8(bin_data.to_vec()).ok()
                        }
                    } else {
                        String::from_utf8(bin_data.to_vec()).ok()
                    }
                }
                Ok(Message::Close(_)) => {
                    is_running_input.store(false, std::sync::atomic::Ordering::Relaxed);
                    break;
                }
                Err(e) => {
                    eprintln!("WebSocket read error: {:?}", e);
                    is_running_input.store(false, std::sync::atomic::Ordering::Relaxed);
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
                                let resp = match mirror_core::file_manager::FileManager::list_directory(&p) {
                                    Ok((canonical_path, entries)) => serde_json::to_string(&mirror_core::file_manager::FsListResponse {
                                        msg_type: "fs_list_resp".to_string(),
                                        id,
                                        path: canonical_path,
                                        entries,
                                    }).unwrap(),
                                    Err(e) => serde_json::to_string(&mirror_core::file_manager::FsErrorResponse {
                                        msg_type: "fs_error".to_string(),
                                        id,
                                        error: e,
                                    }).unwrap(),
                                };
                                let mut sender = ws_sender_input.lock().await;
                                let _ = sender.send(tokio_tungstenite::tungstenite::Message::Text(resp.into())).await;
                            }
                            ClientInput::FsReadChunk { id, path, offset, length } => {
                                let len = length.unwrap_or(65536);
                                let resp = match mirror_core::file_manager::FileManager::read_chunk(&path, offset, len) {
                                    Ok(mut r) => {
                                        r.id = id;
                                        serde_json::to_string(&r).unwrap()
                                    }
                                    Err(e) => serde_json::to_string(&mirror_core::file_manager::FsErrorResponse {
                                        msg_type: "fs_error".to_string(),
                                        id,
                                        error: e,
                                    }).unwrap(),
                                };
                                let mut sender = ws_sender_input.lock().await;
                                let _ = sender.send(tokio_tungstenite::tungstenite::Message::Text(resp.into())).await;
                            }
                            ClientInput::FsWriteChunk { id, path, offset, data_b64, eof } => {
                                let resp = match mirror_core::file_manager::FileManager::write_chunk(&path, offset, &data_b64, eof) {
                                    Ok(mut r) => {
                                        r.id = id;
                                        serde_json::to_string(&r).unwrap()
                                    }
                                    Err(e) => serde_json::to_string(&mirror_core::file_manager::FsErrorResponse {
                                        msg_type: "fs_error".to_string(),
                                        id,
                                        error: e,
                                    }).unwrap(),
                                };
                                let mut sender = ws_sender_input.lock().await;
                                let _ = sender.send(tokio_tungstenite::tungstenite::Message::Text(resp.into())).await;
                            }
                            ClientInput::FsMkdir { id, path } => {
                                let resp = match mirror_core::file_manager::FileManager::create_dir(&path) {
                                    Ok(_) => serde_json::to_string(&mirror_core::file_manager::FsActionResponse {
                                        msg_type: "fs_action_resp".to_string(),
                                        id,
                                        action: "mkdir".to_string(),
                                        success: true,
                                    }).unwrap(),
                                    Err(e) => serde_json::to_string(&mirror_core::file_manager::FsErrorResponse {
                                        msg_type: "fs_error".to_string(),
                                        id,
                                        error: e,
                                    }).unwrap(),
                                };
                                let mut sender = ws_sender_input.lock().await;
                                let _ = sender.send(tokio_tungstenite::tungstenite::Message::Text(resp.into())).await;
                            }
                            ClientInput::FsDelete { id, path, is_dir } => {
                                let resp = match mirror_core::file_manager::FileManager::delete_item(&path, is_dir) {
                                    Ok(_) => serde_json::to_string(&mirror_core::file_manager::FsActionResponse {
                                        msg_type: "fs_action_resp".to_string(),
                                        id,
                                        action: "delete".to_string(),
                                        success: true,
                                    }).unwrap(),
                                    Err(e) => serde_json::to_string(&mirror_core::file_manager::FsErrorResponse {
                                        msg_type: "fs_error".to_string(),
                                        id,
                                        error: e,
                                    }).unwrap(),
                                };
                                let mut sender = ws_sender_input.lock().await;
                                let _ = sender.send(tokio_tungstenite::tungstenite::Message::Text(resp.into())).await;
                            }
                            ClientInput::SystemSas => {
                                println!("[Input] Received system_sas command");
                                let pipe_res = mirror_core::service_manager::send_pipe_command(r#"{"cmd":"sas"}"#).await;
                                let success = if pipe_res.is_ok() {
                                    true
                                } else {
                                    mirror_core::service_manager::trigger_sas().is_ok()
                                };
                                let resp = serde_json::json!({
                                    "type": "system_sas_result",
                                    "success": success,
                                }).to_string();
                                let mut sender = ws_sender_input.lock().await;
                                let _ = sender.send(tokio_tungstenite::tungstenite::Message::Text(resp.into())).await;
                            }
                            ClientInput::SystemElevate => {
                                println!("[Input] Received system_elevate command");
                                let curr_exe = std::env::current_exe().unwrap_or_default();
                                let exe_str = curr_exe.to_str().unwrap_or("");
                                let res = mirror_core::service_manager::request_elevation(Some(exe_str), Some("--elevated"));
                                let resp = serde_json::json!({
                                    "type": "system_elevate_result",
                                    "success": res.is_ok(),
                                    "error": res.err(),
                                }).to_string();
                                let mut sender = ws_sender_input.lock().await;
                                let _ = sender.send(tokio_tungstenite::tungstenite::Message::Text(resp.into())).await;
                            }
                            ClientInput::ServiceStatus => {
                                let status = mirror_core::service_manager::get_service_status();
                                let resp = serde_json::json!({
                                    "type": "service_status_result",
                                    "installed": status.installed,
                                    "running": status.running,
                                    "elevated": status.elevated,
                                    "is_service": status.is_service,
                                }).to_string();
                                let mut sender = ws_sender_input.lock().await;
                                let _ = sender.send(tokio_tungstenite::tungstenite::Message::Text(resp.into())).await;
                            }
                            ClientInput::SystemDesktopSwitch => {
                                println!("[Input] Received system_desktop_switch command");
                                let res = mirror_core::service_manager::switch_to_input_desktop();
                                let resp = serde_json::json!({
                                    "type": "system_desktop_switch_result",
                                    "success": res.is_ok(),
                                    "error": res.err(),
                                }).to_string();
                                let mut sender = ws_sender_input.lock().await;
                                let _ = sender.send(tokio_tungstenite::tungstenite::Message::Text(resp.into())).await;
                            }
                        }
                    } else {
                        eprintln!("Failed to parse input: {}", text);
                    }
                }
            }
        is_running_input.store(false, std::sync::atomic::Ordering::Relaxed);
    });

    let _ = tokio::join!(frame_task, clipboard_task, audio_task, input_task);

    Ok(())
}
