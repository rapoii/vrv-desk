use std::net::SocketAddr;
use std::time::Duration;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

use mirror_core::gdi_capture::ScreenCapturer;
use mirror_core::platform::windows_input::inject_input;
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
    println!("🖥️ Primary Screen detected: {}x{}", test_cap.screen_width, test_cap.screen_height);

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

async fn handle_connection(stream: TcpStream) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ws_stream = accept_async(stream).await?;
    let mut capturer = ScreenCapturer::new()?;
    let screen_w = capturer.screen_width;
    let screen_h = capturer.screen_height;

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(2);
    let shutdown_tx_clone = shutdown_tx.clone();

    // 1. Task: Stream JPEG frames at ~30 FPS
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
                            if ws_sender.send(Message::Binary(jpeg_bytes.into())).await.is_err() {
                                let _ = shutdown_tx.send(()).await;
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

    // 2. Task: Receive and inject input events
    let input_task = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(input) = serde_json::from_str::<ClientInput>(&text) {
                        println!("📥 Received input event: {:?}", input);
                        let event = match input {
                            ClientInput::MouseMove { x, y } => Some(InputEvent::MouseMove { x, y }),
                            ClientInput::MouseDown { button } => {
                                let btn = match button.as_deref() {
                                    Some("right") => MouseButton::Right,
                                    Some("middle") => MouseButton::Middle,
                                    _ => MouseButton::Left,
                                };
                                Some(InputEvent::MouseDown { x: 0.0, y: 0.0, button: btn })
                            }
                            ClientInput::MouseUp { button } => {
                                let btn = match button.as_deref() {
                                    Some("right") => MouseButton::Right,
                                    Some("middle") => MouseButton::Middle,
                                    _ => MouseButton::Left,
                                };
                                Some(InputEvent::MouseUp { x: 0.0, y: 0.0, button: btn })
                            }
                            ClientInput::TouchTap { x, y } => Some(InputEvent::TouchTap { x, y }),
                            ClientInput::MouseWheel { delta_y } => Some(InputEvent::MouseWheel { delta_y }),
                            ClientInput::KeyDown { keycode } => Some(InputEvent::KeyDown { keycode }),
                            ClientInput::KeyUp { keycode } => Some(InputEvent::KeyUp { keycode }),
                        };

                        if let Some(ev) = event {
                            let _ = inject_input(&ev, screen_w as u32, screen_h as u32);
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

    let _ = tokio::join!(frame_task, input_task);

    Ok(())
}
