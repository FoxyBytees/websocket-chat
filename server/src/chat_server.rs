/*use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use log::{debug, error, info};
use protocol::error::Error;
use protocol::*;
use std::time::SystemTime;
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, accept_async, connect_async,
    tungstenite::{self, client::IntoClientRequest},
};

pub struct ChatServer {
    to_tx_handler: Option<Sender<Message>>,
}

impl ChatServer {
    pub fn new() -> Self {
        Self { token: None }
    }

    pub async fn listen<A>(&mut self, address: A) -> Result<(), Error>
    where
        A: ToSocketAddrs,
    {
        let listener = TcpListener::bind(address).await?;

        // Wait for incoming TCP connection
        while let Ok((stream, _)) = listener.accept().await {
            info!("Client connected from {:?}", stream.peer_addr());

            tokio::spawn(connection_handler(
                stream,
                from_req_handler,
                to_req_handler.clone(),
            ));
        }

        Ok(())
    }

    pub async fn wait_done(&mut self) -> Result<(), JoinError> {
        self.chat_msg_join_handle.as_mut().unwrap().await?;
        self.tx_join_handle.as_mut().unwrap().await?;
        self.rx_join_handle.as_mut().unwrap().await?;

        Ok(())
    }
}

async fn request_handler(from_con_handler: Receiver<Message>, to_con_handler: Sender<Message>) {}

async fn connection_handler(
    tcp_stream: TcpStream,
    from_req_handler: Receiver<Message>,
    to_req_handler: Sender<Message>,
) {
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
*/
