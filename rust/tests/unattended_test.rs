use futures_util::{SinkExt, StreamExt};
use mirror_core::auth::{AuthGatekeeper, AuthMessage};
use mirror_core::unattended::UnattendedConfig;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

#[test]
fn test_unattended_config_hash_and_verify() {
    let mut cfg = UnattendedConfig::default();
    assert!(!cfg.enabled);
    assert!(!cfg.verify("Secret123"));

    cfg.set_password("Secret123");
    assert!(cfg.enabled);
    assert!(cfg.salt.is_some());
    assert!(cfg.password_hash.is_some());

    // Correct password
    assert!(cfg.verify("Secret123"));

    // Wrong password
    assert!(!cfg.verify("WrongPass"));
    assert!(!cfg.verify("secret123")); // Case sensitive
    assert!(!cfg.verify(""));

    // Serialization / Deserialization
    let json = serde_json::to_string(&cfg).unwrap();
    let deserialized: UnattendedConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(cfg, deserialized);
    assert!(deserialized.verify("Secret123"));

    // Disabling
    cfg.enabled = false;
    assert!(!cfg.verify("Secret123"));

    // Clearing
    cfg.clear_password();
    assert!(!cfg.enabled);
    assert!(cfg.salt.is_none());
    assert!(cfg.password_hash.is_none());
    assert!(!cfg.verify("Secret123"));
}

async fn start_unattended_host(pin: &'static str, cfg: UnattendedConfig) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let ws_stream = tokio_tungstenite::accept_async(stream).await.unwrap();
            let (mut ws_sender, mut ws_receiver) = ws_stream.split();
            let cfg_clone = cfg.clone();

            tokio::spawn(async move {
                let auth_result = AuthGatekeeper::authenticate_stream_with_unattended(
                    &mut ws_sender,
                    &mut ws_receiver,
                    pin,
                    "Unattended-Host-PC",
                    Some(&cfg_clone),
                )
                .await;

                if auth_result.is_ok() {
                    let _ = ws_sender
                        .send(Message::Text("AUTH_SUCCESS".to_string().into()))
                        .await;
                }
            });
        }
    });

    addr
}

#[tokio::test]
async fn test_auth_with_unattended_static_password() {
    let pin = "123456";
    let mut cfg = UnattendedConfig::default();
    cfg.set_password("MyPermanentPass2026");

    let addr = start_unattended_host(pin, cfg).await;
    let url = format!("ws://{}/", addr);

    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();

    // 1. Receive AuthRequired
    let first_msg = tokio::time::timeout(Duration::from_secs(5), read.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(matches!(first_msg, Message::Text(_)));

    // 2. Client authenticates using unattended permanent password instead of PIN
    let auth_verify = serde_json::json!({
        "type": "auth_verify",
        "pin": "MyPermanentPass2026",
        "e2ee": true
    });
    write
        .send(Message::Text(auth_verify.to_string().into()))
        .await
        .unwrap();

    // 3. Receive AuthOk
    let second_msg = tokio::time::timeout(Duration::from_secs(5), read.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    if let Message::Text(text) = second_msg {
        let auth_ok: AuthMessage = serde_json::from_str(&text).unwrap();
        match auth_ok {
            AuthMessage::AuthOk { e2ee, unattended, .. } => {
                assert!(e2ee);
                assert!(unattended);
            }
            _ => panic!("Expected AuthOk"),
        }
    } else {
        panic!("Expected text message");
    }

    // 4. Stream unlocked
    let unlocked_msg = tokio::time::timeout(Duration::from_secs(5), read.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(unlocked_msg, Message::Text("AUTH_SUCCESS".into()));
}

#[tokio::test]
async fn test_auth_unattended_allows_dynamic_pin_as_well() {
    let pin = "987654";
    let mut cfg = UnattendedConfig::default();
    cfg.set_password("MyPermanentPass2026");

    let addr = start_unattended_host(pin, cfg).await;
    let url = format!("ws://{}/", addr);

    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();

    let _ = read.next().await; // AuthRequired

    // Send the dynamic PIN
    let auth_verify = serde_json::json!({
        "type": "auth_verify",
        "pin": "987654",
        "e2ee": false
    });
    write
        .send(Message::Text(auth_verify.to_string().into()))
        .await
        .unwrap();

    let second_msg = read.next().await.unwrap().unwrap();
    if let Message::Text(text) = second_msg {
        let auth_ok: AuthMessage = serde_json::from_str(&text).unwrap();
        match auth_ok {
            AuthMessage::AuthOk { unattended, .. } => {
                // Was authenticated via dynamic PIN, not unattended password
                assert!(!unattended);
            }
            _ => panic!("Expected AuthOk"),
        }
    }
}

#[tokio::test]
async fn test_auth_unattended_rejects_wrong_password() {
    let pin = "112233";
    let mut cfg = UnattendedConfig::default();
    cfg.set_password("SuperSecret999");

    let addr = start_unattended_host(pin, cfg).await;
    let url = format!("ws://{}/", addr);

    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();

    let _ = read.next().await; // AuthRequired

    // Send incorrect password
    let auth_verify = serde_json::json!({
        "type": "auth_verify",
        "pin": "WrongPasswordAttempt",
        "e2ee": false
    });
    write
        .send(Message::Text(auth_verify.to_string().into()))
        .await
        .unwrap();

    let second_msg = read.next().await.unwrap().unwrap();
    if let Message::Text(text) = second_msg {
        let auth_fail: AuthMessage = serde_json::from_str(&text).unwrap();
        match auth_fail {
            AuthMessage::AuthFailed { remaining_attempts, .. } => {
                assert_eq!(remaining_attempts, 2);
            }
            _ => panic!("Expected AuthFailed"),
        }
    }
}
