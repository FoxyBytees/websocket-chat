use futures_util::{
    SinkExt, StreamExt,
    future::Map,
    stream::{ForEach, SplitSink, SplitStream},
};
use log::{debug, error, info};
use protocol::error::Error;
use protocol::*;
use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr},
    sync::{Arc, RwLock},
    time::SystemTime,
};
use tokio::{
    net::{TcpListener, TcpStream, ToSocketAddrs},
    sync::Mutex,
    task::JoinHandle,
};
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task::JoinError,
};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, accept_async, connect_async,
    tungstenite::{self, Utf8Bytes, client::IntoClientRequest},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

struct AccountInfo {
    username: String,
    password: String,
    ip_address: Option<SocketAddr>,
    token: Option<String>,
}

pub struct ChatServer {
    connections: Arc<RwLock<HashMap<String, AccountInfo>>>,
    cancellation_token: CancellationToken,
    listen_join_handle: Option<JoinHandle<()>>,
}

impl ChatServer {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            cancellation_token: CancellationToken::new(),
            listen_join_handle: None,
        }
    }

    pub fn listen<A>(&mut self, address: A) -> Result<&mut Self, Error>
    where
        A: ToSocketAddrs + std::marker::Send + 'static,
    {
        self.listen_join_handle = Some(tokio::spawn(listen_handler(
            address,
            self.connections.clone(),
            self.cancellation_token.clone(),
        )));

        Ok(self)
    }

    pub fn quit(&mut self) -> Result<&mut Self, Error> {
        self.cancellation_token.cancel();

        Ok(self)
    }

    pub fn is_done(&mut self) -> bool {
        let listen_join_handle = self.listen_join_handle.as_mut();

        listen_join_handle.is_none() || listen_join_handle.unwrap().is_finished()
    }

    pub async fn wait_done(&mut self) -> Result<(), JoinError> {
        if !self.is_done() {
            if let Some(listen_join_handle) = self.listen_join_handle.as_mut() {
                if let Err(err) = listen_join_handle.await {
                    if !err.is_cancelled() {
                        return Err(err);
                    }
                }
            }
        }

        Ok(())
    }
}

async fn listen_handler<A>(
    address: A,
    connections: Arc<RwLock<HashMap<String, AccountInfo>>>,
    cancellation_token: CancellationToken,
) where
    A: ToSocketAddrs,
{
    let mut connection_join_handles: Vec<JoinHandle<()>> = Vec::new();

    match TcpListener::bind(address).await {
        Ok(listener) => {
            while !cancellation_token.is_cancelled() {
                // Wait for incoming TCP connection
                if let Ok((tcp_stream, _)) = listener.accept().await {
                    connection_join_handles.push(tokio::spawn(connection_handler(
                        tcp_stream,
                        connections.clone(),
                        cancellation_token.child_token(),
                    )));
                }
            }
        }
        Err(err) => error!("listen_handler: {}", err),
    }

    for handle in connection_join_handles {
        if let Err(err) = handle.await {
            error!("listen_handler: {}", err);
        }
    }
}

async fn connection_handler(
    tcp_stream: TcpStream,
    connections: Arc<RwLock<HashMap<String, AccountInfo>>>,
    cancellation_token: CancellationToken,
) {
    let ip_address = match tcp_stream.peer_addr() {
        Ok(ip_address) => {
            info!("connection_handler: Client connected from {:?}", ip_address);
            ip_address
        }
        Err(err) => {
            error!("connection_handler: {}", err);
            return;
        }
    };

    let mut ws_stream = match accept_async(tcp_stream).await {
        Ok(ws_stream) => ws_stream,
        Err(err) => {
            error!("Error during the WebSocket handshake: {}", err);
            return;
        }
    };

    loop {
        tokio::select! {
            _ = cancellation_token.cancelled() => return,
            rx_result = ws_stream.next() => {
                if rx_result.is_none() {
                    break;
                }

                // Process websocket data
                match rx_result.unwrap() {
                    Ok(tungstenite::Message::Text(text)) => {
                        debug!("connection_handler: Recv: {}", text);

                        // Deserialize received message
                        match serde_json::from_str::<Message>(text.as_str()) {
                            Ok(message) => match message {
                                Message::UserRegisterRequest(user_register_req) => {
                                    let user_register_res = match connections.write() {
                                        Ok(mut write_guard) => match write_guard.insert(
                                            user_register_req.username.clone(),
                                                AccountInfo {
                                                    username: user_register_req.username,
                                                    password: user_register_req.password,
                                                    ip_address: Some(ip_address),
                                                    token: None
                                                }
                                            ) {
                                            Some(_) => UserRegisterResponse { error: None },
                                            None => UserRegisterResponse { error: Some(String::from("Could not create account")) },
                                        }
                                        Err(err) => UserRegisterResponse { error: Some(err.to_string()) }
                                    };

                                    // Send response
                                    if let Err(err) = ws_stream
                                        .send(tungstenite::Message::Text(Utf8Bytes::from(
                                            serde_json::to_string(&Message::UserRegisterResponse(user_register_res)).unwrap(),
                                        )))
                                        .await
                                    {
                                        error!("connection_handler: {}", err);
                                    }
                                }
                                Message::UserLoginRequest(user_login_req) => {
                                    let user_login_res = match connections.write() {
                                        Ok(mut write_guard) => match write_guard.get_mut(&user_login_req.username) {
                                                Some(account_info) => {
                                                    account_info.token = Some(Uuid::new_v4().to_string());
                                                    UserLoginResponse {token: account_info.token.clone(), error:None }
                                                },
                                                None => UserLoginResponse { token: None, error: Some(String::from("Invalid credentials")) },
                                            },
                                        Err(err) => UserLoginResponse { token: None, error: Some(err.to_string()) },
                                    };

                                    // Send response
                                    if let Err(err) = ws_stream
                                        .send(tungstenite::Message::Text(Utf8Bytes::from(
                                            serde_json::to_string(&Message::UserLoginResponse(user_login_res)).unwrap(),
                                        )))
                                        .await
                                    {
                                        error!("connection_handler: {}", err);
                                    }
                                }
                                Message::ChatMessageRequest(chat_msg_req) => {}
                                _ => {}
                            },
                            Err(err) => error!("connection_handler: {}", err),
                        }
                    }
                    Ok(tungstenite::Message::Close(_)) => {
                        info!("connection_handler: Client closed connection");
                        break;
                    }
                    Err(e) => {
                        error!("connection_handler: {}", e);
                        break;
                    }
                    _ => {} // Ignore other message types
                }
            }
        }
    }

    info!("connection_handler: Shutting down");
}
