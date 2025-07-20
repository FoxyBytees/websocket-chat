use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use log::{debug, error, info};
use protocol::error::Error;
use protocol::*;
use std::time::SystemTime;
use tokio::{net::TcpStream, sync::oneshot, task::JoinHandle};
use tokio::{
    sync::mpsc::{self},
    task::JoinError,
};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{self, client::IntoClientRequest},
};

pub struct ChatClient {
    chat_msg_join_handle: Option<JoinHandle<()>>,
    tx_join_handle: Option<JoinHandle<()>>,
    rx_join_handle: Option<JoinHandle<()>>,
    to_tx_handler: Option<mpsc::Sender<Message>>,
    from_handler: Option<mpsc::Receiver<Result<Message, Error>>>,
    token: Option<String>,
}

impl ChatClient {
    pub fn new() -> Self {
        Self {
            chat_msg_join_handle: None,
            tx_join_handle: None,
            rx_join_handle: None,
            to_tx_handler: None,
            from_handler: None,
            token: None,
        }
    }

    pub async fn connect<R, F>(&mut self, request: R, on_receive: F) -> Result<&mut Self, Error>
    where
        R: IntoClientRequest + Unpin,
        F: Fn(ChatMessage) + std::marker::Send + 'static,
    {
        if !self.is_done() {
            return Err(Error::Connected);
        }

        let ws_stream = connect_async(request).await?.0;
        let split_stream = ws_stream.split();

        // Sender needs to be cloned, Receiver needs to be moved
        let (close_tx_handler, tx_close_signal) = oneshot::channel::<()>();
        let (close_rx_handler, rx_close_signal) = oneshot::channel::<()>();

        let (to_tx_handler, from_method) = mpsc::channel::<Message>(32);
        let (to_method, from_handler) = mpsc::channel::<Result<Message, Error>>(32);
        let (to_chat_msg_handler, chat_msg_receiver) = mpsc::channel::<Message>(32);

        self.from_handler = Some(from_handler);
        self.to_tx_handler = Some(to_tx_handler.clone());

        self.chat_msg_join_handle = Some(tokio::spawn(chat_msg_handler(
            chat_msg_receiver,
            on_receive,
        )));
        self.tx_join_handle = Some(tokio::spawn(tx_handler(
            split_stream.0,
            tx_close_signal,
            close_rx_handler,
            from_method,
            to_method.clone(),
        )));
        self.rx_join_handle = Some(tokio::spawn(rx_handler(
            split_stream.1,
            rx_close_signal,
            close_tx_handler,
            to_method,
            to_chat_msg_handler,
        )));

        Ok(self)
    }

    pub async fn disconnect(&mut self) -> Result<&mut Self, Error> {
        if self.is_done() {
            return Err(Error::Disconnected);
        }

        let to_tx_handler = self.to_tx_handler.clone().unwrap();
        let from_handler = self.from_handler.as_mut().unwrap();

        to_tx_handler.send(Message::Disconnect).await?;

        match from_handler.recv().await {
            Some(Ok(Message::Disconnect)) => Ok(self),
            Some(Err(err)) => Err(err),
            Some(_) => Err(Error::InvalidMessage),
            None => Err(Error::InvalidMessage),
        }
    }

    pub async fn register(&mut self, username: &str, password: &str) -> Result<&mut Self, Error> {
        if self.is_done() {
            return Err(Error::Disconnected);
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
            Some(Ok(Message::UserRegisterResponse(user_register_resp))) => {
                if let Some(desc) = user_register_resp.error {
                    Err(Error::ServerError(desc))
                } else {
                    Ok(self)
                }
            }
            Some(Err(err)) => Err(err),
            Some(_) => Err(Error::InvalidMessage),
            None => Err(Error::InvalidMessage),
        }
    }

    pub async fn login(&mut self, username: &str, password: &str) -> Result<&mut Self, Error> {
        if self.is_done() {
            return Err(Error::Disconnected);
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
            Some(Ok(Message::UserLoginResponse(user_login_resp))) => {
                if let Some(desc) = user_login_resp.error {
                    Err(Error::ServerError(desc))
                } else if let Some(token) = user_login_resp.token {
                    self.token = Some(token);
                    Ok(self)
                } else {
                    Err(Error::InvalidMessage)
                }
            }
            Some(Err(err)) => Err(err),
            Some(_) => Err(Error::InvalidMessage),
            None => Err(Error::InvalidMessage),
        }
    }

    pub async fn message(&mut self, dest_user: &str, message: &str) -> Result<&mut Self, Error> {
        if self.is_done() {
            return Err(Error::Disconnected);
        }

        if self.token.is_none() {
            return Err(Error::Unauthenticated);
        }

        let to_tx_handler = self.to_tx_handler.clone().unwrap();
        let from_handler = self.from_handler.as_mut().unwrap();
        let token = self.token.as_ref().unwrap();

        let chat_msg_req = ChatMessageRequest {
            token: token.clone(),
            dest_user: String::from(dest_user),
            send_time: SystemTime::now(),
            content: String::from(message),
        };

        to_tx_handler
            .send(Message::ChatMessageRequest(chat_msg_req))
            .await?;

        match from_handler.recv().await {
            Some(Ok(Message::ChatMessageResponse(chat_msg_resp))) => {
                if let Some(desc) = chat_msg_resp.error {
                    Err(Error::ServerError(desc))
                } else {
                    Ok(self)
                }
            }
            Some(Err(err)) => Err(err),
            Some(_) => Err(Error::InvalidMessage),
            None => Err(Error::InvalidMessage),
        }
    }

    pub fn is_done(&mut self) -> bool {
        let chat_msg_join_handle = self.chat_msg_join_handle.as_mut();
        let tx_join_handle = self.tx_join_handle.as_mut();
        let rx_join_handle = self.rx_join_handle.as_mut();

        (chat_msg_join_handle.is_none() || chat_msg_join_handle.unwrap().is_finished())
            && (tx_join_handle.is_none() || tx_join_handle.unwrap().is_finished())
            && (rx_join_handle.is_none() || rx_join_handle.unwrap().is_finished())
    }

    pub async fn wait_done(&mut self) -> Result<(), JoinError> {
        if !self.is_done() {
            if let Some(chat_msg_join_handle) = self.chat_msg_join_handle.as_mut() {
                if let Err(err) = chat_msg_join_handle.await {
                    if !err.is_cancelled() {
                        return Err(err);
                    }
                }
            };

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
    mut tx_close_signal: oneshot::Receiver<()>,
    close_rx_handler: oneshot::Sender<()>,
    mut from_method: mpsc::Receiver<Message>,
    to_method: mpsc::Sender<Result<Message, Error>>,
) {
    loop {
        tokio::select! {
        _ = &mut tx_close_signal => break,
        received = from_method.recv() => {
                if received.is_none() {
                    break;
                }

                match received.unwrap() {
                    Message::Disconnect => {
                        info!("tx_handler: Closing connection");

                        // Sending CloseFrame is handled by tungstenite
                        match tx_stream.close().await {
                            Ok(_) => {
                                // Handshake successful, notify method
                                if let Err(err) = to_method.send(Ok(Message::Disconnect)).await {
                                    error!("tx_handler: {}", err);
                                }
                            }
                            Err(err) => {
                                // Error while closing, notify method
                                if let Err(err) = to_method.send(Err(Error::Tungstenite(err))).await {
                                    error!("tx_handler: {}", err);
                                }
                            }
                        };

                        // Signal rx_handler to shut down
                        if close_rx_handler.send(()).is_err() {
                            error!("rx_handler: Could signal tx_handler to stop");
                        }

                        break;
                    }
                    message => {
                        // Serialize message to be sent
                        let json_string = serde_json::to_string(&message).unwrap();
                        debug!("tx_handler: Send: {}", json_string);
                        let message = tungstenite::Message::text(json_string);

                        // Sending message to server
                        if let Err(err) = tx_stream.send(message).await {
                            if let Err(err) = to_method.send(Err(Error::Tungstenite(err))).await {
                                error!("tx_handler: {}", err); // Error while writing to tx_stream, notify method
                            }
                            break;
                        };
                    }
                }
            }
        }
    }

    info!("tx_handler: Shutting down");
}

async fn rx_handler(
    mut rx_stream: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    mut rx_close_signal: oneshot::Receiver<()>,
    close_tx_handler: oneshot::Sender<()>,
    to_method: mpsc::Sender<Result<Message, Error>>,
    to_chat_msg_handler: mpsc::Sender<Message>,
) {
    loop {
        tokio::select! {
            _ = &mut rx_close_signal => break,
            received = rx_stream.next() => {
                if received.is_none() {
                    break;
                }

                match received.unwrap() {
                    Ok(tungstenite::Message::Text(text)) => {
                        debug!("rx_handler: Recv: {}", text);

                        // Deserialize received message
                        match serde_json::from_str::<Message>(text.as_str()) {
                            Ok(message) => {
                                // Check if message is text message meant for chat
                                if let Message::ChatMessage(_) = message {
                                    // Send to chat message receiver
                                    if let Err(err) = to_chat_msg_handler.send(message).await {
                                        error!("rx_handler: {}", err);
                                    }
                                }
                                // Send message back to method
                                else if let Err(err) = to_method.send(Ok(message)).await {
                                    error!("rx_handler: {}", err);
                                }
                            }
                            Err(err) => error!("rx_handler: {}", err), // Error while deserializing, no one to notify
                        }
                    }
                    Ok(tungstenite::Message::Close(_)) => {
                        debug!("rx_handler: Recv CloseFrame");

                        // Signal tx_handler to shut down
                        if close_tx_handler.send(()).is_err() {
                            error!("rx_handler: Could signal tx_handler to stop");
                        }
                        break;
                    }
                    Err(err) => {
                        error!("rx_handler: {}", err); // Error while reading rx_stream
                        break;
                    }
                    _ => debug!("rx_handler: Recv: Unknown message type, ignoring"), // Ignore unknown message types
                }
            }
        }
    }

    // Notify chat_msg_handler to shut down
    if let Err(err) = to_chat_msg_handler.send(Message::Disconnect).await {
        error!("rx_handler: {}", err);
    }

    info!("rx_handler: Shutting down");
}

async fn chat_msg_handler<F>(mut chat_msg_receiver: mpsc::Receiver<Message>, on_chat_msg_recv: F)
where
    F: Fn(ChatMessage),
{
    while let Some(message) = chat_msg_receiver.recv().await {
        match message {
            Message::ChatMessage(chat_msg) => on_chat_msg_recv(chat_msg),
            Message::Disconnect => break,
            _ => debug!("chat_msg_handler: Recv: Unknown message type, ignoring"), // Ignore unknown message types
        }
    }

    info!("chat_msg_handler: Shutting down");
}
