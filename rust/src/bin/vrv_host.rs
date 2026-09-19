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

    let test_cap = ScreenCapturer::new()?;
    println!(
        "🖥️ Primary Screen detected: {}x{}",
        test_cap.screen_width, test_cap.screen_height
    );

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
    let ws_stream = accept_async(stream).await?;
    let capturer = ScreenCapturer::new()?;
    let screen_w = capturer.screen_width;
    let screen_h = capturer.screen_height;

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(2);
    let shutdown_tx_clone = shutdown_tx.clone();
    let shutdown_tx_clip = shutdown_tx.clone();

    let (out_msg_tx, mut out_msg_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

    // 1. Task: Outgoing WebSocket message sender (handles both binary frames and text clipboard sync)
    let send_task = tokio::spawn(async move {
        while let Some(msg) = out_msg_rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                let _ = shutdown_tx.send(()).await;
                break;
            }
        }
    });

    let out_msg_tx_frames = out_msg_tx.clone();
    // 2. Task: Stream JPEG frames at ~30 FPS
    let frame_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(35));
        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    break;
                }
                _ = interval.tick() => {
                    match capturer.capture_jpeg(60, 1024) {
                        Ok(jpeg_bytes) => {
                            if out_msg_tx_frames.send(Message::Binary(jpeg_bytes.into())).is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            eprintln!("Capture error: {}", e);
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                    }
                }
            }
        }
    });

    // 3. Task: Clipboard monitor loop
    // Checks GetClipboardSequenceNumber() every ~500ms.
    // If changed, sends {"type": "clipboard_sync", "text": "..."} to connected client.
    let out_msg_tx_clip = out_msg_tx.clone();
    let clipboard_task = tokio::spawn(async move {
        let mut last_seq = get_clipboard_sequence_number();
        let mut interval = tokio::time::interval(Duration::from_millis(500));
        loop {
            interval.tick().await;
            let current_seq = get_clipboard_sequence_number();
            if current_seq != last_seq {
                last_seq = current_seq;
                if let Some(text) = get_clipboard_text() {
                    let sync_msg = serde_json::json!({
                        "type": "clipboard_sync",
                        "text": text
                    });
                    if out_msg_tx_clip
                        .send(Message::Text(sync_msg.to_string().into()))
                        .is_err()
                    {
                        let _ = shutdown_tx_clip.send(()).await;
                        break;
                    }
                }
            }
        }
    });

    // 4. Task: Receive and inject input events
    let input_task = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
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
                    let _ = shutdown_tx_clone.send(()).await;
                    break;
                }
                Err(e) => {
                    eprintln!("WebSocket read error: {:?}", e);
                    let _ = shutdown_tx_clone.send(()).await;
                    break;
                }
                _ => {}
            }
        }
    });

    let _ = tokio::select! {
        _ = frame_task => {},
        _ = input_task => {},
        _ = clipboard_task => {},
        _ = send_task => {},
    };

    Ok(())
}
