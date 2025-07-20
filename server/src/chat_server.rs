use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use log::{debug, error, info};
use protocol::error::Error;
use protocol::*;
use std::{sync::Arc, time::SystemTime};
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
    tungstenite::{self, client::IntoClientRequest},
};

pub struct ChatServer {
    connections: Arc<Mutex<Vec<i32>>>,
    listen_join_handle: Option<JoinHandle<()>>,
}

impl ChatServer {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(vec![])),
            listen_join_handle: None,
        }
    }

    pub async fn listen<A>(&mut self, address: A) -> Result<&mut Self, Error>
    where
        A: ToSocketAddrs + std::marker::Send + 'static,
    {
        self.listen_join_handle = Some(tokio::spawn(listen_handler(address)));
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
            };
        }

        Ok(())
    }
}

async fn listen_handler<A>(address: A)
where
    A: ToSocketAddrs,
{
    match TcpListener::bind(address).await {
        Ok(listener) => {
            // Wait for incoming TCP connection
            while let Ok((stream, _)) = listener.accept().await {
                info!(
                    "listen_handler: Client connected from {:?}",
                    stream.peer_addr()
                );

                tokio::spawn(connection_handler(stream));
            }
        }
        Err(err) => error!("listen_handler: {}", err),
    }
}

async fn connection_handler(tcp_stream: TcpStream) {
    let mut ws_stream = match accept_async(tcp_stream).await {
        Ok(ws_stream) => ws_stream,
        Err(err) => {
            error!("Error during the WebSocket handshake: {}", err);
            return;
        }
    };

    // Process data in loop
    while let Some(rx_result) = ws_stream.next().await {
        match rx_result {
            Ok(tungstenite::Message::Text(text)) => {
                debug!("connection_handler: Recv: {}", text);

                // Deserialize received message
                match serde_json::from_str::<Message>(text.as_str()) {
                    Ok(message) => match message {
                        Message::UserRegisterRequest(user_register_req) => {}
                        Message::UserLoginRequest(user_login_req) => {}
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

    info!("connection_handler: Shutting down");
}
