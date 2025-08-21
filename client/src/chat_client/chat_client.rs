use crate::chat_client::client_error::ClientError;
use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use log::{debug, error, info};
use protocol::*;
use std::time::SystemTime;
use tokio::{net::TcpStream, task::JoinHandle};
use tokio::{
    sync::mpsc::{self},
    task::JoinError,
};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{self, client::IntoClientRequest},
};
use tokio_util::sync::CancellationToken;

pub struct ChatClient {
    token: Option<String>,
    to_tx_handler: Option<mpsc::Sender<Message>>,
    from_handler: Option<mpsc::Receiver<Result<Message, ClientError>>>,
    tx_join_handle: Option<JoinHandle<()>>,
    rx_join_handle: Option<JoinHandle<()>>,
    cancellation_token: CancellationToken,
}

impl ChatClient {
    pub fn new() -> Self {
        Self {
            token: None,
            to_tx_handler: None,
            from_handler: None,
            tx_join_handle: None,
            rx_join_handle: None,
            cancellation_token: CancellationToken::new(),
        }
    }

    pub async fn connect<R, F>(
        &mut self,
        request: R,
        on_chat_msg_receive: F,
    ) -> Result<&mut Self, ClientError>
    where
        R: IntoClientRequest + Unpin,
        F: Fn(ChatMessage) + Send + 'static,
    {
        if !self.is_done() {
            return Err(ClientError::Connected);
        }

        let ws_stream = connect_async(request).await?.0;
        let split_stream = ws_stream.split();

        // Sender needs to be cloned, Receiver needs to be moved
        let (to_tx_handler, from_method) = mpsc::channel::<Message>(32);
        let (to_method, from_handler) = mpsc::channel::<Result<Message, ClientError>>(32);

        self.from_handler = Some(from_handler);
        self.to_tx_handler = Some(to_tx_handler);

        self.tx_join_handle = Some(tokio::spawn(tx_handler(
            split_stream.0,
            from_method,
            to_method.clone(),
            self.cancellation_token.clone(),
        )));
        self.rx_join_handle = Some(tokio::spawn(rx_handler(
            split_stream.1,
            to_method,
            on_chat_msg_receive,
            self.cancellation_token.clone(),
        )));

        Ok(self)
    }

    pub async fn disconnect(&mut self) -> Result<&mut Self, ClientError> {
        if self.is_done() {
            return Err(ClientError::Disconnected);
        }

        let from_handler = self.from_handler.as_mut().unwrap();

        self.cancellation_token.cancel();

        match from_handler.recv().await {
            Some(Ok(Message::DisconnectSuccess)) => Ok(self),
            Some(Err(err)) => Err(err),
            Some(_) => Err(ClientError::InvalidMessage),
            None => Err(ClientError::InvalidMessage),
        }
    }

    pub async fn register(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<&mut Self, ClientError> {
        if self.is_done() {
            return Err(ClientError::Disconnected);
        }

        let to_tx_handler = self.to_tx_handler.clone().unwrap();
        let from_handler = self.from_handler.as_mut().unwrap();

        let user_register_req = UserRegisterRequest {
            username: String::from(username),
            password: String::from(password),
        };

        to_tx_handler
            .send(Message::UserRegisterRequest(user_register_req))
            .await?;

        match from_handler.recv().await {
            Some(Ok(Message::UserRegisterResponse(user_register_res))) => {
                if let Some(desc) = user_register_res.error {
                    Err(ClientError::ServerError(desc))
                } else {
                    Ok(self)
                }
            }
            Some(Err(err)) => Err(err),
            Some(_) => Err(ClientError::InvalidMessage),
            None => Err(ClientError::InvalidMessage),
        }
    }

    pub async fn login(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<&mut Self, ClientError> {
        if self.is_done() {
            return Err(ClientError::Disconnected);
        }

        let to_tx_handler = self.to_tx_handler.clone().unwrap();
        let from_handler = self.from_handler.as_mut().unwrap();

        let user_login_req = UserLoginRequest {
            username: String::from(username),
            password: String::from(password),
        };

        to_tx_handler
            .send(Message::UserLoginRequest(user_login_req))
            .await?;

        match from_handler.recv().await {
            Some(Ok(Message::UserLoginResponse(user_login_res))) => {
                if let Some(desc) = user_login_res.error {
                    Err(ClientError::ServerError(desc))
                } else if let Some(token) = user_login_res.session_token {
                    self.token = Some(token);
                    Ok(self)
                } else {
                    Err(ClientError::InvalidMessage)
                }
            }
            Some(Err(err)) => Err(err),
            Some(_) => Err(ClientError::InvalidMessage),
            None => Err(ClientError::InvalidMessage),
        }
    }

    pub async fn message(
        &mut self,
        dest_user: &str,
        message: &str,
    ) -> Result<&mut Self, ClientError> {
        if self.is_done() {
            return Err(ClientError::Disconnected);
        }

        if self.token.is_none() {
            return Err(ClientError::Unauthenticated);
        }

        let to_tx_handler = self.to_tx_handler.clone().unwrap();
        let from_handler = self.from_handler.as_mut().unwrap();
        let token = self.token.as_ref().unwrap();

        let chat_msg_req = ChatMessageRequest {
            session_token: token.clone(),
            dest_user: String::from(dest_user),
            send_time: SystemTime::now(),
            content: String::from(message),
        };

        to_tx_handler
            .send(Message::ChatMessageRequest(chat_msg_req))
            .await?;

        match from_handler.recv().await {
            Some(Ok(Message::ChatMessageResponse(chat_msg_res))) => {
                if let Some(desc) = chat_msg_res.error {
                    Err(ClientError::ServerError(desc))
                } else {
                    Ok(self)
                }
            }
            Some(Err(err)) => Err(err),
            Some(_) => Err(ClientError::InvalidMessage),
            None => Err(ClientError::InvalidMessage),
        }
    }

    pub fn is_done(&mut self) -> bool {
        let tx_join_handle = self.tx_join_handle.as_mut();
        let rx_join_handle = self.rx_join_handle.as_mut();

        (tx_join_handle.is_none() || tx_join_handle.unwrap().is_finished())
            && (rx_join_handle.is_none() || rx_join_handle.unwrap().is_finished())
    }

    pub async fn wait_done(&mut self) -> Result<(), JoinError> {
        if !self.is_done() {
            if let Some(tx_join_handle) = self.tx_join_handle.as_mut() {
                if let Err(err) = tx_join_handle.await {
                    if !err.is_cancelled() {
                        return Err(err);
                    }
                }
            };

            if let Some(rx_join_handle) = self.rx_join_handle.as_mut() {
                if let Err(err) = rx_join_handle.await {
                    if !err.is_cancelled() {
                        return Err(err);
                    }
                }
            };
        }

        Ok(())
    }
}

async fn tx_handler(
    mut tx_stream: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, tungstenite::Message>,
    mut from_method: mpsc::Receiver<Message>,
    to_method: mpsc::Sender<Result<Message, ClientError>>,
    cancellation_token: CancellationToken,
) {
    loop {
        tokio::select! {
        _ = cancellation_token.cancelled() => {
            // Sending CloseFrame is handled by tungstenite
            match tx_stream.close().await {
                Ok(_) => {
                    // Handshake successful, notify method
                    if let Err(err) = to_method.send(Ok(Message::DisconnectSuccess)).await {
                        error!("tx_handler: {}", err);
                    }
                }
                Err(err) => {
                    // Error while closing, notify method
                    if let Err(err) = to_method.send(Err(ClientError::Tungstenite(err))).await {
                        error!("tx_handler: {}", err);
                    }
                }
            };

            break;
        },
        received = from_method.recv() => {
                match received {
                    Some(message) => {
                        // Serialize message to be sent
                        let json_string = serde_json::to_string(&message).unwrap();
                        debug!("tx_handler: send: {}", json_string);
                        let message = tungstenite::Message::text(json_string);

                        // Sending message to server
                        if let Err(err) = tx_stream.send(message).await {
                            if let Err(err) = to_method.send(Err(ClientError::Tungstenite(err))).await {
                                error!("tx_handler: {}", err); // Error while writing to tx_stream, notify method
                            }
                            break;
                        };
                    },
                    None => break,
                }
            }
        }
    }

    info!("tx_handler: shutting down");
}

async fn rx_handler<F>(
    mut rx_stream: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    to_method: mpsc::Sender<Result<Message, ClientError>>,
    on_chat_msg_recv: F,
    cancellation_token: CancellationToken,
) where
    F: Fn(ChatMessage),
{
    loop {
        tokio::select! {
            _ = cancellation_token.cancelled() => break,
            received = rx_stream.next() => {
                match received {
                    Some(Ok(tungstenite::Message::Text(text))) => {
                        debug!("rx_handler: recv: {}", text);

                        // Deserialize received message
                        match serde_json::from_slice::<Message>(text.as_bytes()) {
                            Ok(Message::ChatMessage(chat_msg) ) => on_chat_msg_recv(chat_msg),
                            Ok(message) => {
                                // Send back to method
                                if let Err(err) = to_method.send(Ok(message)).await {
                                    error!("rx_handler: {}", err);
                                }
                            }
                            Err(err) => error!("rx_handler: {}", err), // Error while deserializing, no one to notify
                        }
                    }
                    Some(Ok(tungstenite::Message::Close(_))) => {
                        debug!("rx_handler: received CloseFrame");
                        break;
                    }
                    Some(Err(err)) => {
                        error!("rx_handler: {}", err); // Error while reading rx_stream
                        break;
                    }
                    None => break,
                    _ => debug!("rx_handler: received unknown message type, ignoring"), // Ignore unknown message types
                }
            }
        }
    }

    info!("rx_handler: shutting down");
}