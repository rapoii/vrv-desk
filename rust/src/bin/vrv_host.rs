use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

use mirror_core::gdi_capture::ScreenCapturer;
use mirror_core::platform::windows_input::{
    get_clipboard_sequence_number, get_clipboard_text, inject_input, inject_shortcut,
    inject_unicode_text, set_clipboard_text,
};
use mirror_core::protocol::{InputEvent, MouseButton};

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = "0.0.0.0:53211".parse()?;
    let listener = TcpListener::bind(&addr).await?;

    println!("==================================================");
    println!("⚡ VrV Desk Host Engine running on {}", addr);
    println!("Ready for Android & remote streaming connections...");
    println!("==================================================");

    let screen_w = unsafe { windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(windows::Win32::UI::WindowsAndMessaging::SM_CXSCREEN) };
    let screen_h = unsafe { windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(windows::Win32::UI::WindowsAndMessaging::SM_CYSCREEN) };
    println!("🖥️ Primary Screen detected: {}x{}", screen_w, screen_h);

    while let Ok((stream, peer_addr)) = listener.accept().await {
        println!("🔗 New client connected from: {}", peer_addr);

        let _ = stream.set_nodelay(true);

        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream).await {
                eprintln!("Connection error with {}: {:?}", peer_addr, e);
            }
            println!("🔌 Client {} disconnected.", peer_addr);
        });
    }

    Ok(())
}

async fn handle_connection(
    stream: TcpStream,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🤝 Starting WebSocket handshake...");
    let ws_stream = accept_async(stream).await?;
    println!("✅ WebSocket handshake completed!");
    let capturer = ScreenCapturer::new().map_err(|e| format!("Capturer init failed: {}", e))?;
    println!("✅ Screen capturer initialized!");
    let screen_w = capturer.screen_width;
    let screen_h = capturer.screen_height;

    let (ws_sender, mut ws_receiver) = ws_stream.split();
    let ws_sender = std::sync::Arc::new(tokio::sync::Mutex::new(ws_sender));

    let is_running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));

    let is_running_frame = is_running.clone();
    let ws_sender_frame = ws_sender.clone();
    // 1. Task: Stream JPEG frames at ~30 FPS
    let frame_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(35));
        while is_running_frame.load(std::sync::atomic::Ordering::Relaxed) {
            interval.tick().await;
            match capturer.capture_jpeg(60, 1024) {
                Ok(jpeg_bytes) => {
                    let mut sender = ws_sender_frame.lock().await;
                    if sender.send(Message::Binary(jpeg_bytes.into())).await.is_err() {
                        is_running_frame.store(false, std::sync::atomic::Ordering::Relaxed);
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("Capture error: {}", e);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    });

    // 2. Task: Clipboard monitor loop
    let is_running_clip = is_running.clone();
    let ws_sender_clip = ws_sender.clone();
    let clipboard_task = tokio::spawn(async move {
        let mut last_seq = get_clipboard_sequence_number();
        let mut interval = tokio::time::interval(Duration::from_millis(500));
        while is_running_clip.load(std::sync::atomic::Ordering::Relaxed) {
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

    // 3. Task: Receive and inject input events
    let is_running_input = is_running.clone();
    let input_task = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            if !is_running_input.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(input) = serde_json::from_str::<ClientInput>(&text) {
                        println!("📥 Received input event: {:?}", input);
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
                                set_clipboard_text(&text);
                            }
                        }
                    } else {
                        eprintln!("Failed to parse input: {}", text);
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
                _ => {}
            }
        }
        is_running_input.store(false, std::sync::atomic::Ordering::Relaxed);
    });

    let _ = tokio::join!(frame_task, clipboard_task, input_task);

    Ok(())
}
