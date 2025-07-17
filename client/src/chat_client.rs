use futures_util::{SinkExt, StreamExt, stream::FusedStream};
use protocol::error::Error;
use protocol::*;
use std::time::SystemTime;
use tokio::net::TcpStream;
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
    websocket_stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    token: Option<String>,
}

impl ChatClient {
    pub fn new() -> Self {
        Self {
            websocket_stream: None,
            token: None,
        }
    }

    pub async fn connect<R>(&mut self, request: R) -> Result<(), Error>
    where
        R: IntoClientRequest + Unpin,
    {
        let result = connect_async(request).await?;
        self.websocket_stream = Some(result.0);

        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<(), Error> {
        if let Some(mut stream) = self.websocket_stream.take() {
            if stream.is_terminated() {
                return Err(Error::Disconnected(String::from("Already terminated")));
            }

            stream
                .close(Some(CloseFrame {
                    code: CloseCode::Normal,
                    reason: Utf8Bytes::from(String::from("Disconnected")),
                }))
                .await?;
            self.websocket_stream = None;
        }

        Ok(())
    }

    pub async fn register(&mut self, username: &str, password: &str) -> Result<(), Error> {
        if let Some(stream) = self.websocket_stream.as_mut() {
            if stream.is_terminated() {
                return Err(Error::Disconnected(String::from("Could not register")));
            }

            let register_request = protocol::RegisterRequest {
                username: String::from(username),
                password: String::from(password),
            };
            let json = serde_json::to_string(&register_request).unwrap();

            stream.send(Message::Text(Utf8Bytes::from(json))).await?;

            match stream.next().await {
                Some(response_result) => {
                    let register_response = serde_json::from_str::<RegisterResponse>(
                        response_result?.into_text()?.as_str(),
                    )?;

                    match register_response.error {
                        Some(desc) => Err(Error::RegisterFailed(desc)),
                        None => Ok(()),
                    }
                }
                None => Err(Error::RegisterFailed(String::from("No response"))),
            }
        } else {
            Err(Error::Disconnected(String::from("Could not register")))
        }
    }

    pub async fn login(&mut self, username: &str, password: &str) -> Result<(), Error> {
        if let Some(stream) = self.websocket_stream.as_mut() {
            if stream.is_terminated() {
                return Err(Error::Disconnected(String::from("Could not login")));
            }

            let login_request = protocol::LoginRequest {
                username: String::from(username),
                password: String::from(password),
            };
            let json = serde_json::to_string(&login_request).unwrap();

            stream.send(Message::Text(Utf8Bytes::from(json))).await?;

            match stream.next().await {
                Some(response_result) => {
                    let login_response = serde_json::from_str::<LoginResponse>(
                        response_result?.into_text()?.as_str(),
                    )?;

                    match login_response.error {
                        Some(desc) => Err(Error::LoginFailed(desc)),
                        None => match login_response.token {
                            Some(token) => {
                                self.token = Some(token);
                                Ok(())
                            }
                            None => Err(Error::LoginFailed(String::from("No token"))),
                        },
                    }
                }
                None => Err(Error::LoginFailed(String::from("No response"))),
            }
        } else {
            Err(Error::Disconnected(String::from("Could not login")))
        }
    }

    pub async fn message(&mut self, dest_user: &str, message: &str) -> Result<SystemTime, Error> {
        if let Some(stream) = self.websocket_stream.as_mut() {
            if stream.is_terminated() {
                return Err(Error::Disconnected(String::from("Could not send message")));
            }

            if let Some(token) = &self.token {
                let login_request = protocol::MessageRequest {
                    token: token.to_string(),
                    dest_user: String::from(dest_user),
                    send_time: SystemTime::now(),
                    content: String::from(message),
                };

                let json = serde_json::to_string(&login_request).unwrap();

                stream.send(Message::Text(Utf8Bytes::from(json))).await?;

                match stream.next().await {
                    Some(response_result) => {
                        let message_response = serde_json::from_str::<MessageResponse>(
                            response_result?.into_text()?.as_str(),
                        )?;

                        match message_response.error {
                            Some(desc) => Err(Error::MessageFailed(desc)),
                            None => Ok(message_response.arrive_time),
                        }
                    }
                    None => Err(Error::LoginFailed(String::from("No response"))),
                }
            } else {
                Err(Error::MessageFailed(String::from("Please login")))
            }
        } else {
            Err(Error::Disconnected(String::from("Could not send message")))
        }
    }
}
