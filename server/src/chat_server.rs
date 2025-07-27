mod account_manager;
use crate::chat_server::account_manager::AccountManager;

use log::{debug, error, info};
use protocol::error::Error;
use protocol::*;
use std::sync::{Arc, RwLock};
use tokio::task::JoinError;
use tokio::{
    net::{TcpListener, TcpStream, ToSocketAddrs},
    task::JoinHandle,
};
use tokio_tungstenite::{
    accept_async,
    tungstenite::{self, Utf8Bytes},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

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

    pub fn listen<A>(&mut self, address: A) -> Result<&mut Self, Error>
    where
        A: ToSocketAddrs + std::marker::Send + 'static,
    {
        self.listen_join_handle = Some(tokio::spawn(listen_handler(
            address,
            self.account_manager.clone(),
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
    account_manager: AccountManager,
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
                        account_manager.clone(),
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
    account_manager: AccountManager,
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

    let split_stream = ws_stream.split();
    let mut tx_stream = split_stream.0;
    let mut rx_stream = split_stream.1;

    loop {
        /*tokio::select! {
        _ = cancellation_token.cancelled() => return,
        rx_result = ws_stream.next() => {*/
        let rx_result = rx_stream.next().await;

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
                            // TODO: AccountManager::create_account()

                            // Send response
                            if let Err(err) = tx_stream
                                .send(tungstenite::Message::Text(Utf8Bytes::from(
                                    serde_json::to_string(&Message::UserRegisterResponse(
                                        user_register_res,
                                    ))
                                    .unwrap(),
                                )))
                                .await
                            {
                                error!("connection_handler: {}", err);
                            }
                        }
                        Message::UserLoginRequest(user_login_req) => {
                            let user_login_res = match accounts.write() {
                                Ok(mut accounts) => {
                                    match accounts.get_mut(&user_login_req.username) {
                                        Some(account) => {
                                            // Check if password is valid
                                            if account.password == user_login_req.password {
                                                let new_token = Uuid::new_v4().to_string();

                                                match sessions.write() {
                                                    // Create new session
                                                    Ok(mut sessions) => match sessions.insert(
                                                        new_token.clone(),
                                                        Session {
                                                            username: user_login_req
                                                                .username
                                                                .clone(),
                                                            ws_tx_stream: Arc::new(RwLock::new(
                                                                tx_stream,
                                                            )),
                                                        },
                                                    ) {
                                                        // Could not create session (Session already exists)
                                                        Some(_) => UserLoginResponse {
                                                            session_token: None,
                                                            error: Some(String::from(
                                                                "Could not create account",
                                                            )),
                                                        },
                                                        // Successfully created session
                                                        None => UserLoginResponse {
                                                            session_token: Some(new_token),
                                                            error: None,
                                                        },
                                                    },
                                                    // Could not create session (RwLock poisoned)
                                                    Err(err) => UserLoginResponse {
                                                        session_token: None,
                                                        error: Some(err.to_string()),
                                                    },
                                                }
                                            } else {
                                                // Password is invalid
                                                UserLoginResponse {
                                                    session_token: None,
                                                    error: Some(String::from(
                                                        "Invalid credentials",
                                                    )),
                                                }
                                            }
                                        }
                                        // Username is invalid
                                        None => UserLoginResponse {
                                            session_token: None,
                                            error: Some(String::from("Invalid credentials")),
                                        },
                                    }
                                }
                                Err(err) => UserLoginResponse {
                                    session_token: None,
                                    error: Some(err.to_string()),
                                },
                            };

                            // Send response
                            if let Err(err) = ws_stream
                                .send(tungstenite::Message::Text(Utf8Bytes::from(
                                    serde_json::to_string(&Message::UserLoginResponse(
                                        user_login_res,
                                    ))
                                    .unwrap(),
                                )))
                                .await
                            {
                                error!("connection_handler: {}", err);
                            }
                        }
                        Message::ChatMessageRequest(chat_msg_req) => {
                            let chat_msg_res = match sessions.read() {
                                // Get account by session token
                                Ok(sessions) => match sessions.get(&chat_msg_req.session_token) {
                                    // User is authenticated, continue
                                    Some(session) => match accounts.read() {
                                        Ok(accounts) => {
                                            match accounts.get(&chat_msg_req.dest_user) {
                                                // TODO: forward message to dest_user
                                                Some(account) => {
                                                    ChatMessageResponse { error: None }
                                                }

                                                // User does not exist
                                                None => ChatMessageResponse {
                                                    error: Some(String::from(
                                                        "User does not exist",
                                                    )),
                                                },
                                            }
                                        }
                                        // Could not get account (RwLock poisoned)
                                        Err(err) => ChatMessageResponse {
                                            error: Some(err.to_string()),
                                        },
                                    },
                                    // Could not get account (Session key not assigned)
                                    None => ChatMessageResponse {
                                        error: Some(String::from("Session not found")),
                                    },
                                },
                                // Could not get session (RwLock poisoned)
                                Err(err) => ChatMessageResponse {
                                    error: Some(err.to_string()),
                                },
                            };

                            // Send response
                            if let Err(err) = ws_stream
                                .send(tungstenite::Message::Text(Utf8Bytes::from(
                                    serde_json::to_string(&Message::ChatMessageResponse(
                                        chat_msg_res,
                                    ))
                                    .unwrap(),
                                )))
                                .await
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
        //}
        //}
    }

    info!("connection_handler: Shutting down");
}
