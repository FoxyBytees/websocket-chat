use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use protocol::error::Error;
use protocol::*;
use std::time::SystemTime;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{
        self, Utf8Bytes, client::IntoClientRequest, protocol::CloseFrame,
        protocol::frame::coding::CloseCode,
    },
};
use tungstenite::protocol::Message;

pub struct ChatClient {
    //TODO: Implement message queue in separate thread and notify methods if response arrives, use timout
    to_tx_handler: Option<Sender<Message>>,
    from_rx_handler: Option<Receiver<Result<tungstenite::Message, tungstenite::Error>>>,
    token: Option<String>,
}

impl ChatClient {
    pub fn new() -> Self {
        Self {
            to_tx_handler: None,
            from_rx_handler: None,
            token: None,
        }
    }

    pub async fn connect<R>(&mut self, request: R) -> Result<(), Error>
    where
        R: IntoClientRequest + Unpin,
    {
        let connect_result = connect_async(request).await?;
        let split_stream = connect_result.0.split();

        // Sender needs to be cloned, Receiver needs to be moved
        let (to_tx_handler, from_method) = mpsc::channel::<tungstenite::Message>(32);
        let (to_disconnect_method, from_rx_handler) =
            mpsc::channel::<Result<tungstenite::Message, tungstenite::Error>>(32);

        self.from_rx_handler = Some(from_rx_handler);
        self.to_tx_handler = Some(to_tx_handler);

        tokio::spawn(rx_handler(split_stream.1, to_disconnect_method));
        tokio::spawn(tx_handler(split_stream.0, from_method));

        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<(), Error> {
        if let Some(to_tx_handler) = self.to_tx_handler.clone() {
            if let Some(from_rx_handler) = self.from_rx_handler.as_mut() {
                to_tx_handler
                    .send(Message::Close(Some(CloseFrame {
                        code: CloseCode::Normal,
                        reason: Utf8Bytes::from(String::from("Disconnected")),
                    })))
                    .await?;

                if let Some(rx_result) = from_rx_handler.recv().await {
                    match rx_result {
                        Ok(_) => {
                            self.from_rx_handler = None;
                            self.to_tx_handler = None;

                            return Ok(());
                        }
                        Err(err) => return Err(Error::Tungstenite(err)),
                    }
                }
            }
        }

        Err(Error::Disconnected(String::from("Please connect first")))
    }

    pub async fn register(&mut self, username: &str, password: &str) -> Result<(), Error> {
        if let Some(to_tx_handler) = self.to_tx_handler.clone() {
            if let Some(from_rx_handler) = self.from_rx_handler.as_mut() {
                let register_request = RegisterRequest {
                    username: String::from(username),
                    password: String::from(password),
                };
                let json = serde_json::to_string(&register_request).unwrap();

                to_tx_handler
                    .send(Message::Text(Utf8Bytes::from(json)))
                    .await?;

                if let Some(rx_result) = from_rx_handler.recv().await {
                    match rx_result {
                        Ok(msg) => {
                            let register_response = serde_json::from_str::<RegisterResponse>(
                                msg.into_text()?.as_str(),
                            )?;

                            match register_response.error {
                                Some(desc) => return Err(Error::RegisterFailed(desc)),
                                None => return Ok(()),
                            }
                        }
                        Err(err) => return Err(Error::Tungstenite(err)),
                    }
                }
            }
        }

        Err(Error::Disconnected(String::from("Please connect first")))
    }

    pub async fn login(&mut self, username: &str, password: &str) -> Result<(), Error> {
        if let Some(to_tx_handler) = self.to_tx_handler.clone() {
            if let Some(from_rx_handler) = self.from_rx_handler.as_mut() {
                let login_request = LoginRequest {
                    username: String::from(username),
                    password: String::from(password),
                };
                let json = serde_json::to_string(&login_request).unwrap();

                to_tx_handler
                    .send(Message::Text(Utf8Bytes::from(json)))
                    .await?;

                if let Some(rx_result) = from_rx_handler.recv().await {
                    match rx_result {
                        Ok(msg) => {
                            let login_response =
                                serde_json::from_str::<LoginResponse>(msg.into_text()?.as_str())?;

                            match login_response.error {
                                Some(desc) => return Err(Error::LoginFailed(desc)),
                                None => match login_response.token {
                                    Some(token) => {
                                        self.token = Some(token);
                                        return Ok(());
                                    }
                                    None => {
                                        return Err(Error::LoginFailed(String::from("No token")));
                                    }
                                },
                            };
                        }
                        Err(err) => return Err(Error::Tungstenite(err)),
                    }
                }
            }
        }

        Err(Error::Disconnected(String::from("Please connect first")))
    }

    pub async fn message(&mut self, dest_user: &str, message: &str) -> Result<SystemTime, Error> {
        if let Some(to_tx_handler) = self.to_tx_handler.clone() {
            if let Some(from_rx_handler) = self.from_rx_handler.as_mut() {
                if let Some(token) = &self.token {
                    let login_request = MessageRequest {
                        token: token.to_string(),
                        dest_user: String::from(dest_user),
                        send_time: SystemTime::now(),
                        content: String::from(message),
                    };

                    let json = serde_json::to_string(&login_request).unwrap();

                    to_tx_handler
                        .send(Message::Text(Utf8Bytes::from(json)))
                        .await?;

                    if let Some(rx_result) = from_rx_handler.recv().await {
                        match rx_result {
                            Ok(msg) => {
                                let message_response = serde_json::from_str::<MessageResponse>(
                                    msg.into_text()?.as_str(),
                                )?;

                                match message_response.error {
                                    Some(desc) => return Err(Error::MessageFailed(desc)),
                                    None => match message_response.arrive_time {
                                        Some(arrive_time) => {
                                            return Ok(arrive_time);
                                        }
                                        None => {
                                            return Err(Error::MessageFailed(String::from(
                                                "No arrive time",
                                            )));
                                        }
                                    },
                                };
                            }
                            Err(err) => return Err(Error::Tungstenite(err)),
                        }
                    }
                } else {
                    return Err(Error::MessageFailed(String::from("Please login")));
                }
            }
        }

        Err(Error::Disconnected(String::from("Please connect first")))
    }
}

async fn rx_handler(
    mut rx_stream: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    to_method: Sender<Result<tungstenite::Message, tungstenite::Error>>,
) {
    loop {
        if let Some(msg_result) = rx_stream.next().await {
            match msg_result {
                Ok(msg) => {
                    if (to_method.send(Ok(msg.clone())).await).is_err() {
                        break;
                    }

                    if let Message::Close(_) = msg {
                        break;
                    }
                }
                Err(err) => {
                    if (to_method.send(Err(err)).await).is_err() {
                        break;
                    }
                }
            }
        }
    }
}

async fn tx_handler(
    mut tx_stream: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    mut from_method: Receiver<Message>,
) {
    loop {
        if let Some(msg) = from_method.recv().await {
            if (tx_stream.send(msg.clone()).await).is_err() {
                let _ = tx_stream.close().await;
                break;
            };

            let _ = tx_stream.flush().await;

            if let Message::Close(_) = msg {
                let _ = tx_stream.close().await;
                break;
            }
        }
    }
}
