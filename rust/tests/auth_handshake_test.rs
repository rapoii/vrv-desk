use futures_util::{SinkExt, StreamExt};
use mirror_core::auth::{AuthGatekeeper, AuthMessage};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

async fn start_mock_host(pin: &'static str) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let ws_stream = tokio_tungstenite::accept_async(stream).await.unwrap();
            let (mut ws_sender, mut ws_receiver) = ws_stream.split();

            tokio::spawn(async move {
                let auth_result = AuthGatekeeper::authenticate_stream(
                    &mut ws_sender,
                    &mut ws_receiver,
                    pin,
                    "Mock-Host-PC",
                )
                .await;

                if auth_result.is_ok() {
                    // Send a dummy payload to signal stream has unlocked
                    let _ = ws_sender
                        .send(Message::Text("STREAM_UNLOCKED".to_string().into()))
                        .await;
                }
            });
        }
    });

    addr
}

#[tokio::test]
async fn test_auth_handshake_successful_pin() {
    let pin = "849201";
    let addr = start_mock_host(pin).await;
    let url = format!("ws://{}/", addr);

    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();

    // 1. Expect AuthRequired
    let first_msg = tokio::time::timeout(Duration::from_secs(5), read.next())
        .await
        .expect("Timeout waiting for initial message")
        .expect("No message")
        .expect("WebSocket error");

    if let Message::Text(text) = first_msg {
        let auth_req: AuthMessage = serde_json::from_str(&text).expect("Valid JSON");
        match auth_req {
            AuthMessage::AuthRequired { host_name, version } => {
                assert_eq!(host_name, "Mock-Host-PC");
                assert_eq!(version, "0.4.0");
            }
            _ => panic!("Expected AuthRequired, got {:?}", auth_req),
        }
    } else {
        panic!("Expected text message");
    }

    // 2. Send correct AuthVerify
    let verify_msg = AuthMessage::AuthVerify {
        pin: pin.to_string(),
        e2ee: false,
    };
    write
        .send(Message::Text(
            serde_json::to_string(&verify_msg).unwrap().into(),
        ))
        .await
        .expect("Failed to send verify");

    // 3. Expect AuthOk
    let ok_msg = tokio::time::timeout(Duration::from_secs(5), read.next())
        .await
        .expect("Timeout waiting for auth_ok")
        .expect("No message")
        .expect("WebSocket error");

    if let Message::Text(text) = ok_msg {
        let auth_resp: AuthMessage = serde_json::from_str(&text).expect("Valid JSON");
        match auth_resp {
            AuthMessage::AuthOk { session_token, .. } => {
                assert_eq!(session_token.len(), 32); // 16 bytes hex = 32 chars
            }
            _ => panic!("Expected AuthOk, got {:?}", auth_resp),
        }
    } else {
        panic!("Expected text message");
    }

    // 4. Verify stream unlock signal is received
    let stream_msg = tokio::time::timeout(Duration::from_secs(5), read.next())
        .await
        .expect("Timeout waiting for unlocked signal")
        .expect("No message")
        .expect("WebSocket error");

    if let Message::Text(text) = stream_msg {
        assert_eq!(text, "STREAM_UNLOCKED");
    } else {
        panic!("Expected STREAM_UNLOCKED");
    }
}

#[tokio::test]
async fn test_auth_handshake_invalid_pin_retry() {
    let correct_pin = "123456";
    let addr = start_mock_host(correct_pin).await;
    let url = format!("ws://{}/", addr);

    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();

    // 1. Consume AuthRequired
    let _ = read.next().await.unwrap().unwrap();

    // 2. Send Wrong PIN 1
    let verify_wrong = AuthMessage::AuthVerify {
        pin: "000000".to_string(),
        e2ee: false,
    };
    write
        .send(Message::Text(
            serde_json::to_string(&verify_wrong).unwrap().into(),
        ))
        .await
        .unwrap();

    let resp_msg = read.next().await.unwrap().unwrap();
    if let Message::Text(text) = resp_msg {
        let auth_resp: AuthMessage = serde_json::from_str(&text).unwrap();
        match auth_resp {
            AuthMessage::AuthFailed {
                remaining_attempts, ..
            } => {
                assert_eq!(remaining_attempts, 2);
            }
            _ => panic!("Expected AuthFailed with 2 attempts left"),
        }
    }

    // 3. Send Wrong PIN 2
    write
        .send(Message::Text(
            serde_json::to_string(&verify_wrong).unwrap().into(),
        ))
        .await
        .unwrap();

    let resp_msg2 = read.next().await.unwrap().unwrap();
    if let Message::Text(text) = resp_msg2 {
        let auth_resp: AuthMessage = serde_json::from_str(&text).unwrap();
        match auth_resp {
            AuthMessage::AuthFailed {
                remaining_attempts, ..
            } => {
                assert_eq!(remaining_attempts, 1);
            }
            _ => panic!("Expected AuthFailed with 1 attempt left"),
        }
    }

    // 4. Now send correct PIN -> Should succeed
    let verify_correct = AuthMessage::AuthVerify {
        pin: correct_pin.to_string(),
        e2ee: false,
    };
    write
        .send(Message::Text(
            serde_json::to_string(&verify_correct).unwrap().into(),
        ))
        .await
        .unwrap();

    let ok_msg = read.next().await.unwrap().unwrap();
    if let Message::Text(text) = ok_msg {
        let auth_resp: AuthMessage = serde_json::from_str(&text).unwrap();
        match auth_resp {
            AuthMessage::AuthOk { session_token, .. } => {
                assert_eq!(session_token.len(), 32);
            }
            _ => panic!("Expected AuthOk"),
        }
    }
}

#[tokio::test]
async fn test_auth_handshake_three_failed_attempts_disconnects() {
    let correct_pin = "654321";
    let addr = start_mock_host(correct_pin).await;
    let url = format!("ws://{}/", addr);

    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();

    // Consume AuthRequired
    let _ = read.next().await.unwrap().unwrap();

    let wrong_msg = AuthMessage::AuthVerify {
        pin: "999999".to_string(),
        e2ee: false,
    };
    let wrong_text = serde_json::to_string(&wrong_msg).unwrap();

    // Attempt 1: remaining = 2
    write
        .send(Message::Text(wrong_text.clone().into()))
        .await
        .unwrap();
    let msg1 = read.next().await.unwrap().unwrap();
    if let Message::Text(text) = msg1 {
        let resp: AuthMessage = serde_json::from_str(&text).unwrap();
        assert_eq!(
            resp,
            AuthMessage::AuthFailed {
                reason: "Invalid PIN".to_string(),
                remaining_attempts: 2,
            }
        );
    }

    // Attempt 2: remaining = 1
    write
        .send(Message::Text(wrong_text.clone().into()))
        .await
        .unwrap();
    let msg2 = read.next().await.unwrap().unwrap();
    if let Message::Text(text) = msg2 {
        let resp: AuthMessage = serde_json::from_str(&text).unwrap();
        assert_eq!(
            resp,
            AuthMessage::AuthFailed {
                reason: "Invalid PIN".to_string(),
                remaining_attempts: 1,
            }
        );
    }

    // Attempt 3: remaining = 0 -> terminal auth_failed and socket closed
    write.send(Message::Text(wrong_text.into())).await.unwrap();
    let msg3 = read.next().await.unwrap().unwrap();
    if let Message::Text(text) = msg3 {
        let resp: AuthMessage = serde_json::from_str(&text).unwrap();
        assert_eq!(
            resp,
            AuthMessage::AuthFailed {
                reason: "Too many failed attempts. Disconnecting.".to_string(),
                remaining_attempts: 0,
            }
        );
    }

    // Socket should now be closed / EOF / Err
    let next_msg = tokio::time::timeout(Duration::from_secs(3), read.next()).await;
    match next_msg {
        Ok(None) => {} // socket cleanly closed
        Ok(Some(Ok(Message::Close(_)))) => {} // received close frame
        Ok(Some(Err(_))) => {} // socket error on close
        other => panic!("Expected socket close, got {:?}", other),
    }
}
