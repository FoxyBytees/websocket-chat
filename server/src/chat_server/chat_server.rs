use crate::chat_server::account_manager::account_manager::AccountManager;
use log::{debug, error, info};
use protocol::{Message};

use futures_util::{SinkExt, StreamExt};
use protocol::*;
use tokio::task::JoinError;
use tokio::{
    net::{TcpListener, TcpStream, ToSocketAddrs},
    task::JoinHandle,
};
use tokio::sync::mpsc;
use tokio_tungstenite::{
    accept_async,
    tungstenite::{self, Utf8Bytes},
};
use tokio_util::sync::CancellationToken;

pub struct ChatServer {
    account_manager: AccountManager,
    cancellation_token: CancellationToken,
    listen_join_handle: Option<JoinHandle<()>>,
}

impl ChatServer {
    pub fn new() -> Self {
        Self {
            account_manager: AccountManager::new(),
            cancellation_token: CancellationToken::new(),
            listen_join_handle: None,
        }
    }

    pub fn listen<A>(&mut self, address: A) -> &mut Self
    where
        A: ToSocketAddrs + Send + 'static,
    {
        self.listen_join_handle = Some(tokio::spawn(listen_handler(
            address,
            self.account_manager.clone(),
            self.cancellation_token.clone(),
        )));

        self
    }

    pub fn stop(&mut self) -> &mut Self {
        self.cancellation_token.cancel();

        self
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
    account_manager: AccountManager,
    cancellation_token: CancellationToken,
) where
    A: ToSocketAddrs,
{
    let mut connection_join_handles: Vec<JoinHandle<()>> = Vec::new();

    match TcpListener::bind(address).await {
        Ok(listener) => loop {
            tokio::select! {
                _ = cancellation_token.cancelled() => break,
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((tcp_stream, _addr)) => connection_join_handles.push(
                            tokio::spawn(connection_handler(
                                tcp_stream,
                                account_manager.clone(),
                                cancellation_token.child_token(),
                            ))),
                        Err(err) => error!("listen_handler: {}", err),
                    };
                }
            }
        },
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
    mut account_manager: AccountManager,
    cancellation_token: CancellationToken,
) {
    match tcp_stream.peer_addr() {
        Ok(ip_address) => {
            info!("connection_handler: Client connected from {:?}", ip_address);
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

    let (to_my_user, mut from_other_user) = mpsc::channel::<Message>(32);

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
                                    let user_register_res = match account_manager.create_account(
                                        user_register_req.username,
                                        user_register_req.password,
                                    ).await {
                                        Ok(_) => UserRegisterResponse { error: None },
                                        Err(err) => {
                                            error!("connection_handler: {}", err);

                                            UserRegisterResponse {
                                                error: Some(err.to_string()),
                                            }
                                        }
                                    };

                                    // Send response
                                    if let Err(err) = ws_stream
                                        .send(tungstenite::Message::Text(Utf8Bytes::from(
                                            serde_json::to_string(&Message::UserRegisterResponse(
                                                user_register_res,
                                            )).unwrap(),
                                        ))).await
                                    {
                                        error!("connection_handler: {}", err);
                                    }
                                }
                                Message::UserLoginRequest(user_login_req) => {
                                    let user_login_res = match account_manager.create_session(
                                        user_login_req.username,
                                        user_login_req.password,
                                        to_my_user.clone()
                                    ).await {
                                        Ok(token) => UserLoginResponse {
                                            session_token: Some(token),
                                            error: None,
                                        },
                                        Err(err) => {
                                            error!("connection_handler: {}", err);

                                            UserLoginResponse {
                                                session_token: None,
                                                error: Some(err.to_string()),
                                            }
                                        }
                                    };

                                    // Send response
                                    if let Err(err) = ws_stream
                                        .send(tungstenite::Message::Text(Utf8Bytes::from(
                                            serde_json::to_string(&Message::UserLoginResponse(
                                                user_login_res,
                                            )).unwrap(),
                                        ))).await
                                    {
                                        error!("connection_handler: {}", err);
                                    }
                                }
                                Message::ChatMessageRequest(chat_msg_req) => {
                                    let chat_msg_res = match account_manager
                                        .get_username_by_token(&chat_msg_req.session_token).await
                                    {
                                        Ok(src_user) => {
                                            match account_manager.get_sender_by_username(&chat_msg_req.dest_user).await {
                                                Ok(sender) => {
                                                    match sender.send(Message::ChatMessage(
                                                        ChatMessage {
                                                            src_user,
                                                            send_time: chat_msg_req.send_time,
                                                            content: chat_msg_req.content,
                                                        },
                                                    )).await {
                                                        Ok(_) => ChatMessageResponse {
                                                            error: None,
                                                        },
                                                        Err(err) => {
                                                            error!("connection_handler: {}", err);

                                                            ChatMessageResponse {
                                                                error: Some(err.to_string()),
                                                            }
                                                        }
                                                    }
                                                },
                                                Err(err) => {
                                                    error!("connection_handler: {}", err);

                                                    ChatMessageResponse {
                                                        error: Some(err.to_string()),
                                                    }
                                                }
                                            }
                                        }
                                        Err(err) => {
                                            error!("connection_handler: {}", err);

                                            ChatMessageResponse {
                                                error: Some(err.to_string()),
                                            }
                                        }
                                    };

                                    // Send response
                                    if let Err(err) = ws_stream
                                        .send(tungstenite::Message::Text(Utf8Bytes::from(
                                            serde_json::to_string(&Message::ChatMessageResponse(
                                                chat_msg_res,
                                            )).unwrap(),
                                        ))).await
                                    {
                                        error!("connection_handler: {}", err);
                                    }
                                }
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
            },
            message = from_other_user.recv() => {
                match message {
                    Some(chat_msg) => {
                        if let Err(err) = ws_stream
                        .send(tungstenite::Message::Text(Utf8Bytes::from(
                            serde_json::to_string(&chat_msg).unwrap(),
                        ))).await
                        {
                            error!("connection_handler: {}", err);
                        }
                    },
                    _ => debug!("chat_msg_handler: Recv: Unknown message type, ignoring"), // Ignore unknown message types
                }
            }
        }
    }

    info!("connection_handler: Shutting down");
}
