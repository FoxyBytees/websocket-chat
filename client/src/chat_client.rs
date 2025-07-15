use protocol::{Error, ErrorKind};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt, future::OkInto};
use serde_json;
use tokio::{
    net::{TcpListener, TcpStream},
    time,
};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{
        self, Utf8Bytes, client::IntoClientRequest, error::UrlError, http::response,
        protocol::CloseFrame,
    },
};
use tungstenite::protocol::Message;

struct ChatClient {
    websocket_stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

impl ChatClient {
    pub fn new() -> Self {
        Self {
            websocket_stream: None,
        }
    }

    pub async fn connect<R>(&mut self, request: R) -> Result<(), Error>
    where
        R: IntoClientRequest + Unpin,
    {
        match connect_async(request).await {
            Ok((stream, _)) => self.websocket_stream = Some(stream),
            Err(error) => {
                return Err(Error {
                    kind: ErrorKind::ConnectionError,
                    error: error.to_string(),
                });
            }
        };

        Ok(())
    }

    pub async fn disconnect(&mut self, msg: Option<CloseFrame>) -> Result<(), Error> {
        if let Some(mut stream) = self.websocket_stream.take() {
            match stream.close(msg).await {
                Ok(_) => {}
                Err(error) => {
                    return Err(Error {
                        kind: ErrorKind::ConnectionError,
                        error: error.to_string(),
                    });
                }
            };
        }

        Ok(())
    }

    pub async fn login(
        &mut self,
        username: String,
        password: String,
        timeout: u64,
    ) -> Result<String, Error> {
        if let Some(stream) = self.websocket_stream.as_mut() {
            let login_request = protocol::LoginRequest { username, password };
            let json = serde_json::to_string(&login_request).unwrap();

            match stream.send(Message::Text(Utf8Bytes::from(json))).await {
                Ok(_) => {}
                Err(error) => {
                    return Err(Error {
                        kind: ErrorKind::ConnectionError,
                        error: error.to_string(),
                    });
                }
            };

            match stream.next().await {
                Some(response) => match response {
                    Ok(message) => {}
                    Err(error) => {
                        return Err(Error {
                            kind: ErrorKind::LoginError,
                            error: error.to_string(),
                        });
                    }
                },
                None => {
                    return Err(Error {
                        kind: ErrorKind::LoginError,
                        error: "No Login Response".to_string(),
                    });
                }
            }
        } else {
            Err(Error {
                kind: ErrorKind::Timeout,
                error: "Socket not connected".to_string(),
            })
        }
    }
}
