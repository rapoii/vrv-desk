//! VrV Desk Remote Signaling Broker
//! High-performance WebSocket server for matching remote hosts and clients across networks.

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{oneshot, Mutex, RwLock};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

pub const DEFAULT_SIGNAL_PORT: u16 = 53212;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum SignalMessage {
    #[serde(rename = "register_host")]
    RegisterHost {
        device_id: String,
        name: Option<String>,
        stun_endpoint: Option<String>,
    },
    #[serde(rename = "register_ok")]
    RegisterOk { device_id: String },
    #[serde(rename = "register_error")]
    RegisterError { reason: String },

    #[serde(rename = "connect_request")]
    ConnectRequest {
        target_id: String,
        client_name: Option<String>,
    },
    #[serde(rename = "connect_error")]
    ConnectError { reason: String },
}

pub struct HostRendezvous {
    pub client_ws: tokio_tungstenite::WebSocketStream<TcpStream>,
    pub connect_req_json: String,
}

/// A pending rendezvous pairing between a registered Host and an incoming Client
#[allow(dead_code)]
pub struct HostEntry {
    pub device_id: String,
    pub name: Option<String>,
    pub stun_endpoint: Option<String>,
    /// Sender to pass the client's WebSocket stream and request to the host handler
    pub rendezvous_tx: Mutex<Option<oneshot::Sender<HostRendezvous>>>,
}

pub type HostMap = Arc<RwLock<HashMap<String, Arc<HostEntry>>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut port = DEFAULT_SIGNAL_PORT;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--port" || arg == "-p" {
            if let Some(val) = args.next() {
                port = val.parse().unwrap_or(DEFAULT_SIGNAL_PORT);
            }
        }
    }

    if let Ok(env_port) = env::var("VRV_SIGNAL_PORT") {
        if let Ok(p) = env_port.parse() {
            port = p;
        }
    }

    let bind_addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&bind_addr).await?;

    println!("=================================================");
    println!("🌐 VrV Desk Signaling Broker");
    println!("📡 Listening on: ws://{}", bind_addr);
    println!("⚡ Ready to route remote peer connections");
    println!("=================================================");

    let host_map: HostMap = Arc::new(RwLock::new(HashMap::new()));

    while let Ok((stream, addr)) = listener.accept().await {
        let hosts = Arc::clone(&host_map);
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, addr, hosts).await {
                eprintln!("[Signal] Connection error from {}: {:?}", addr, e);
            }
        });
    }

    Ok(())
}

/// Handle incoming TCP connection, perform WS upgrade, and route based on the initial message
pub async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    host_map: HostMap,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut ws_stream = accept_async(stream).await?;

    // Read initial greeting / registration / connection request
    let first_msg = match ws_stream.next().await {
        Some(Ok(m)) => m,
        Some(Err(e)) => return Err(e.into()),
        None => return Ok(()),
    };

    let text = match first_msg {
        Message::Text(t) => t,
        _ => return Ok(()),
    };

    let signal_msg: SignalMessage = match serde_json::from_str(&text) {
        Ok(m) => m,
        Err(_) => return Ok(()),
    };

    match signal_msg {
        SignalMessage::RegisterHost {
            device_id,
            name,
            stun_endpoint,
        } => {
            handle_host_registration(
                device_id,
                name,
                stun_endpoint,
                ws_stream,
                host_map,
                addr,
            )
            .await
        }
        SignalMessage::ConnectRequest {
            target_id,
            client_name: _,
        } => {
            handle_client_connect(
                target_id,
                text.to_string(),
                ws_stream,
                host_map,
                addr,
            )
            .await
        }
        _ => Ok(()),
    }
}

/// Handle a host connection registering its device ID and waiting for a client to bridge
async fn handle_host_registration(
    device_id: String,
    name: Option<String>,
    stun_endpoint: Option<String>,
    mut host_ws: tokio_tungstenite::WebSocketStream<TcpStream>,
    host_map: HostMap,
    addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let clean_id = device_id.replace(' ', "");
    println!(
        "[Signal] Host registered: ID={}, Name={:?}, STUN={:?}, Addr={}",
        clean_id, name, stun_endpoint, addr
    );

    let (rendezvous_tx, mut rendezvous_rx) = oneshot::channel();

    let entry = Arc::new(HostEntry {
        device_id: clean_id.clone(),
        name,
        stun_endpoint,
        rendezvous_tx: Mutex::new(Some(rendezvous_tx)),
    });

    {
        let mut map = host_map.write().await;
        map.insert(clean_id.clone(), entry);
    }

    // Send register_ok
    let ok_msg = SignalMessage::RegisterOk {
        device_id: clean_id.clone(),
    };
    host_ws
        .send(Message::Text(serde_json::to_string(&ok_msg)?.into()))
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

    // Wait for client rendezvous or host disconnect / ping
    let rendezvous_opt = loop {
        tokio::select! {
            client_res = &mut rendezvous_rx => {
                match client_res {
                    Ok(r) => break Some(r),
                    Err(_) => break None,
                }
            }
            msg = host_ws.next() => {
                match msg {
                    Some(Ok(Message::Ping(p))) => {
                        let _ = host_ws.send(Message::Pong(p)).await;
                    }
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => {
                        println!("[Signal] Host disconnected: ID={}", clean_id);
                        break None;
                    }
                    _ => {}
                }
            }
        }
    };

    if let Some(rendezvous) = rendezvous_opt {
        println!("[Signal] Bridging Client and Host for Device ID={}", clean_id);
        // Remove from active registry while bridged
        {
            let mut map = host_map.write().await;
            map.remove(&clean_id);
        }

        // Notify host that a remote client has connected and bridging is beginning
        let notify_host = serde_json::json!({
            "type": "client_connected",
            "device_id": clean_id
        });
        if host_ws
            .send(Message::Text(notify_host.to_string().into()))
            .await
            .is_err()
        {
            eprintln!("[Signal] Failed to send client_connected notice to host {}", clean_id);
            return Ok(());
        }

        // Bridge host and client directly
        bridge_websockets(host_ws, rendezvous.client_ws).await;
        println!("[Signal] Session ended for Device ID={}", clean_id);
    }

    // Remove if still present
    {
        let mut map = host_map.write().await;
        map.remove(&clean_id);
    }

    Ok(())
}

/// Handle a client connection requesting connection to a target host ID
async fn handle_client_connect(
    target_id: String,
    raw_req_text: String,
    mut client_ws: tokio_tungstenite::WebSocketStream<TcpStream>,
    host_map: HostMap,
    addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let clean_id = target_id.replace(' ', "");
    println!(
        "[Signal] Client from {} requested connect to target ID={}",
        addr, clean_id
    );

    let host_entry = {
        let map = host_map.read().await;
        map.get(&clean_id).cloned()
    };

    let host_entry = match host_entry {
        Some(entry) => entry,
        None => {
            let err_msg = SignalMessage::ConnectError {
                reason: format!("Host {} not found or offline", clean_id),
            };
            let _ = client_ws
                .send(Message::Text(serde_json::to_string(&err_msg)?.into()))
                .await;
            return Ok(());
        }
    };

    let rendezvous_sender = {
        let mut tx_opt = host_entry.rendezvous_tx.lock().await;
        tx_opt.take()
    };

    match rendezvous_sender {
        Some(tx) => {
            let rendezvous = HostRendezvous {
                client_ws,
                connect_req_json: raw_req_text,
            };
            if tx.send(rendezvous).is_err() {
                eprintln!("[Signal] Failed to hand off client stream to host {}", clean_id);
            }
        }
        None => {
            let err_msg = SignalMessage::ConnectError {
                reason: format!("Host {} is currently busy in another session", clean_id),
            };
            let _ = client_ws
                .send(Message::Text(serde_json::to_string(&err_msg)?.into()))
                .await;
        }
    }

    Ok(())
}

/// Bidirectional bridge between Host and Client WebSocket streams
async fn bridge_websockets(
    host_ws: tokio_tungstenite::WebSocketStream<TcpStream>,
    client_ws: tokio_tungstenite::WebSocketStream<TcpStream>,
) {
    let (mut host_sink, mut host_stream) = host_ws.split();
    let (mut client_sink, mut client_stream) = client_ws.split();

    // Host -> Client forwarder
    let host_to_client = async move {
        while let Some(msg_res) = host_stream.next().await {
            match msg_res {
                Ok(msg) => {
                    let is_close = msg.is_close();
                    if client_sink.send(msg).await.is_err() || is_close {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        let _ = client_sink.close().await;
    };

    // Client -> Host forwarder
    let client_to_host = async move {
        while let Some(msg_res) = client_stream.next().await {
            match msg_res {
                Ok(msg) => {
                    let is_close = msg.is_close();
                    if host_sink.send(msg).await.is_err() || is_close {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        let _ = host_sink.close().await;
    };

    tokio::select! {
        _ = host_to_client => {},
        _ = client_to_host => {},
    }
}
