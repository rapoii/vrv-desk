//! Authentication Gatekeeper & Handshake Types
//! Handles dynamic PIN auth handshake over WebSocket connections.

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(tag = "type")]
pub enum AuthMessage {
    #[serde(rename = "auth_required")]
    AuthRequired {
        host_name: String,
        version: String,
    },
    #[serde(rename = "auth_verify")]
    AuthVerify { pin: String },
    #[serde(rename = "auth_ok")]
    AuthOk { session_token: String },
    #[serde(rename = "auth_failed")]
    AuthFailed {
        reason: String,
        remaining_attempts: u32,
    },
}

pub struct AuthGatekeeper;

impl AuthGatekeeper {
    pub async fn authenticate_stream<S>(
        ws_sender: &mut futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<S>, Message>,
        ws_receiver: &mut futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<S>>,
        expected_pin: &str,
        host_name: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        // 1. Send auth_required message
        let auth_req = AuthMessage::AuthRequired {
            host_name: host_name.to_string(),
            version: "0.4.0".to_string(),
        };
        ws_sender
            .send(Message::Text(serde_json::to_string(&auth_req)?.into()))
            .await?;

        // 2. Auth loop: expect auth_verify, up to 3 attempts, 20s timeout per message
        let mut remaining_attempts: u32 = 3;

        while remaining_attempts > 0 {
            let msg_res = tokio::time::timeout(Duration::from_secs(20), ws_receiver.next()).await;
            let msg = match msg_res {
                Ok(Some(Ok(m))) => m,
                Ok(Some(Err(e))) => {
                    return Err(format!("WebSocket read error during auth: {:?}", e).into());
                }
                Ok(None) => {
                    return Err("WebSocket connection closed by client during auth".into());
                }
                Err(_) => {
                    return Err("Auth handshake timed out (20s)".into());
                }
            };

            match msg {
                Message::Text(text) => {
                    if let Ok(AuthMessage::AuthVerify { pin: client_pin }) =
                        serde_json::from_str::<AuthMessage>(&text)
                    {
                        if client_pin.trim() == expected_pin {
                            let token_bytes: [u8; 16] = rand::random();
                            let session_token = hex::encode(token_bytes);
                            let ok_msg = AuthMessage::AuthOk {
                                session_token: session_token.clone(),
                            };
                            ws_sender
                                .send(Message::Text(serde_json::to_string(&ok_msg)?.into()))
                                .await?;
                            return Ok(session_token);
                        } else {
                            remaining_attempts = remaining_attempts.saturating_sub(1);
                            if remaining_attempts > 0 {
                                let fail_msg = AuthMessage::AuthFailed {
                                    reason: "Invalid PIN".to_string(),
                                    remaining_attempts,
                                };
                                ws_sender
                                    .send(Message::Text(serde_json::to_string(&fail_msg)?.into()))
                                    .await?;
                            } else {
                                let fail_msg = AuthMessage::AuthFailed {
                                    reason: "Too many failed attempts. Disconnecting.".to_string(),
                                    remaining_attempts: 0,
                                };
                                let _ = ws_sender
                                    .send(Message::Text(serde_json::to_string(&fail_msg)?.into()))
                                    .await;
                                let _ = ws_sender.close().await;
                                return Err("Authentication failed: too many invalid attempts".into());
                            }
                        }
                    } else {
                        eprintln!("Unexpected or invalid auth message format: {}", text);
                    }
                }
                Message::Ping(payload) => {
                    let _ = ws_sender.send(Message::Pong(payload)).await;
                }
                Message::Close(_) => {
                    return Err("Client closed connection during auth".into());
                }
                _ => {}
            }
        }

        Err("Authentication rejected".into())
    }
}
