//! Integration tests for Signaling Broker & Host Remote Registration

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

// We can spawn a signaling broker instance directly in test using its functions or TcpListener
#[path = "../src/bin/vrv_signal.rs"]
mod vrv_signal;

#[tokio::test]
async fn test_signaling_host_registration_and_client_connect() {
    let host_map: vrv_signal::HostMap = Arc::new(RwLock::new(HashMap::new()));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let broker_addr = listener.local_addr().unwrap();

    let hosts_clone = Arc::clone(&host_map);
    // Background broker task
    let broker_task = tokio::spawn(async move {
        while let Ok((stream, addr)) = listener.accept().await {
            let hosts = Arc::clone(&hosts_clone);
            tokio::spawn(async move {
                let _ = vrv_signal::handle_connection(stream, addr, hosts).await;
            });
        }
    });

    let broker_ws_url = format!("ws://{}", broker_addr);

    // 1. Host connects and registers
    let (mut host_ws, _) = connect_async(&broker_ws_url).await.unwrap();
    let reg_msg = serde_json::json!({
        "type": "register_host",
        "device_id": "849201",
        "name": "Host-PC-Test",
        "stun_endpoint": "114.10.41.71:31772"
    });
    host_ws
        .send(Message::Text(reg_msg.to_string().into()))
        .await
        .unwrap();

    // Host receives register_ok
    let host_resp = host_ws.next().await.unwrap().unwrap();
    if let Message::Text(txt) = host_resp {
        let val: Value = serde_json::from_str(&txt).unwrap();
        assert_eq!(val["type"], "register_ok");
        assert_eq!(val["device_id"], "849201");
    } else {
        panic!("Expected Text message with register_ok");
    }

    // 2. Client queries unknown device ID -> receives connect_error
    let (mut client_ws_unknown, _) = connect_async(&broker_ws_url).await.unwrap();
    let conn_req_unknown = serde_json::json!({
        "type": "connect_request",
        "target_id": "999999"
    });
    client_ws_unknown
        .send(Message::Text(conn_req_unknown.to_string().into()))
        .await
        .unwrap();

    let err_resp = client_ws_unknown.next().await.unwrap().unwrap();
    if let Message::Text(txt) = err_resp {
        let val: Value = serde_json::from_str(&txt).unwrap();
        assert_eq!(val["type"], "connect_error");
        assert!(val["reason"].as_str().unwrap().contains("not found"));
    } else {
        panic!("Expected connect_error for unknown device ID");
    }

    // Client connects to registered device ID 849201
    let (mut client_ws, _) = connect_async(&broker_ws_url).await.unwrap();
    let conn_req = serde_json::json!({
        "type": "connect_request",
        "target_id": "849 201",
        "client_name": "Test-Client-Phone"
    });
    client_ws
        .send(Message::Text(conn_req.to_string().into()))
        .await
        .unwrap();

    // Give broker a moment to establish bridge
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Host receives client_connected notice from broker
    let host_notice = host_ws.next().await.unwrap().unwrap();
    if let Message::Text(txt) = host_notice {
        let val: Value = serde_json::from_str(&txt).unwrap();
        assert_eq!(val["type"], "client_connected");
        assert_eq!(val["device_id"], "849201");
    } else {
        panic!("Expected client_connected notice from broker");
    }

    // Now broker bridges host_ws and client_ws!
    // Host sends mock auth_required to client
    let auth_req = serde_json::json!({
        "type": "auth_required",
        "host_name": "Host-PC-Test",
        "version": "0.5.0"
    });
    host_ws
        .send(Message::Text(auth_req.to_string().into()))
        .await
        .unwrap();

    // Client receives auth_required
    let client_recv = tokio::time::timeout(Duration::from_secs(2), client_ws.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    if let Message::Text(txt) = client_recv {
        let val: Value = serde_json::from_str(&txt).unwrap();
        assert_eq!(val["type"], "auth_required");
        assert_eq!(val["host_name"], "Host-PC-Test");
    } else {
        panic!("Expected auth_required across bridge");
    }

    // Client sends auth_verify
    let auth_verify = serde_json::json!({
        "type": "auth_verify",
        "pin": "123456"
    });
    client_ws
        .send(Message::Text(auth_verify.to_string().into()))
        .await
        .unwrap();

    // Host receives auth_verify
    let host_recv = tokio::time::timeout(Duration::from_secs(2), host_ws.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    if let Message::Text(txt) = host_recv {
        let val: Value = serde_json::from_str(&txt).unwrap();
        assert_eq!(val["type"], "auth_verify");
        assert_eq!(val["pin"], "123456");
    } else {
        panic!("Expected auth_verify across bridge");
    }

    // Binary payload test (simulating JPEG stream)
    let mock_jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x12, 0x34, 0x56];
    host_ws
        .send(Message::Binary(mock_jpeg.clone().into()))
        .await
        .unwrap();

    let client_frame = tokio::time::timeout(Duration::from_secs(2), client_ws.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    if let Message::Binary(bin) = client_frame {
        assert_eq!(bin.as_ref(), mock_jpeg.as_slice());
    } else {
        panic!("Expected binary frame across bridge");
    }

    broker_task.abort();
}
