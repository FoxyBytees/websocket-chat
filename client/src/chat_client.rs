use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use log::{error, info};
use protocol::error::Error;
use protocol::*;
use std::time::SystemTime;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{self, client::IntoClientRequest},
};

pub struct ChatClient {
    //TODO: Implement message queue in separate thread and notify methods if response arrives, use timout
    to_tx_handler: Option<Sender<Message>>,
    from_handler: Option<Receiver<Result<Message, Error>>>,
    chat_msg_receiver: Option<Receiver<Result<Message, Error>>>,
    token: Option<String>,
}

impl ChatClient {
    pub fn new() -> Self {
        Self {
            to_tx_handler: None,
            from_handler: None,
            chat_msg_receiver: None,
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
        let (to_tx_handler, from_method) = mpsc::channel::<Message>(32);
        let (to_method, from_handler) = mpsc::channel::<Result<Message, Error>>(32);
        let (to_chat_msg_receiver, chat_msg_receiver) = mpsc::channel::<Result<Message, Error>>(32);

        self.from_handler = Some(from_handler);
        self.to_tx_handler = Some(to_tx_handler.clone());
        self.chat_msg_receiver = Some(chat_msg_receiver);

        tokio::spawn(tx_handler(split_stream.0, from_method, to_method.clone()));
        tokio::spawn(rx_handler(
            split_stream.1,
            to_method,
            to_chat_msg_receiver,
            to_tx_handler,
        ));

        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<(), Error> {
        if self.to_tx_handler.is_none() || self.from_handler.is_none() {
            return Err(Error::Disconnected);
        }

        let to_tx_handler = self.to_tx_handler.clone().unwrap();
        let from_handler = self.from_handler.as_mut().unwrap();

        to_tx_handler.send(Message::ClientDisconnect).await?;

        if let Message::ClientDisconnect = from_handler.recv().await.unwrap()? {
            self.from_handler = None;
            self.to_tx_handler = None;

            Ok(())
        } else {
            Err(Error::InvalidMessage)
        }
    }

    pub async fn register(&mut self, username: &str, password: &str) -> Result<(), Error> {
        if self.to_tx_handler.is_none() || self.from_handler.is_none() {
            return Err(Error::Disconnected);
        }

        let to_tx_handler = self.to_tx_handler.clone().unwrap();
        let from_handler = self.from_handler.as_mut().unwrap();

        let register_request = UserRegisterRequest {
            username: String::from(username),
            password: String::from(password),
        };

        to_tx_handler
            .send(Message::UserRegisterRequest(register_request))
            .await?;

        if let Message::UserRegisterResponse(register_response) =
            from_handler.recv().await.unwrap()?
        {
            if let Some(desc) = register_response.error {
                Err(Error::ServerError(desc))
            } else {
                Ok(())
            }
        } else {
            Err(Error::InvalidMessage)
        }
    }

    pub async fn login(&mut self, username: &str, password: &str) -> Result<(), Error> {
        if self.to_tx_handler.is_none() || self.from_handler.is_none() {
            return Err(Error::Disconnected);
        }

        let to_tx_handler = self.to_tx_handler.clone().unwrap();
        let from_handler = self.from_handler.as_mut().unwrap();

        let login_request = UserLoginRequest {
            username: String::from(username),
            password: String::from(password),
        };

        to_tx_handler
            .send(Message::UserLoginRequest(login_request))
            .await?;

        if let Message::UserLoginResponse(login_response) = from_handler.recv().await.unwrap()? {
            if let Some(desc) = login_response.error {
                Err(Error::ServerError(desc))
            } else if let Some(token) = login_response.token {
                self.token = Some(token);
                Ok(())
            } else {
                Err(Error::InvalidMessage)
            }
        } else {
            Err(Error::InvalidMessage)
        }
    }

    pub async fn message(&mut self, dest_user: &str, message: &str) -> Result<(), Error> {
        if self.to_tx_handler.is_none() || self.from_handler.is_none() {
            return Err(Error::Disconnected);
        }

        if self.token.is_none() {
            return Err(Error::Unauthenticated);
        }

        let to_tx_handler = self.to_tx_handler.clone().unwrap();
        let from_handler = self.from_handler.as_mut().unwrap();
        let token = self.token.as_ref().unwrap();

        let chat_msg_request = ChatMessageRequest {
            token: token.clone(),
            dest_user: String::from(dest_user),
            send_time: SystemTime::now(),
            content: String::from(message),
        };

        to_tx_handler
            .send(Message::ChatMessageRequest(chat_msg_request))
            .await?;

        if let Message::ChatMessageResponse(chat_msg_response) =
            from_handler.recv().await.unwrap()?
        {
            if let Some(desc) = chat_msg_response.error {
                Err(Error::ServerError(desc))
            } else {
                Ok(())
            }
        } else {
            Err(Error::InvalidMessage)
        }
    }
}

async fn rx_handler(
    mut rx_stream: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    to_method: Sender<Result<Message, Error>>,
    to_chat_msg_receiver: Sender<Result<Message, Error>>,
    to_tx_handler: Sender<Message>,
) {
    // After stream is closed next() returns Error --> Better solution?
    while let Some(rx_result) = rx_stream.next().await {
        match rx_result {
            Ok(tungstenite::Message::Close(_)) => {
                info!("rx_handler: Server closed connection");

                // Notify tx_handler to shut down
                if let Err(err) = to_tx_handler.send(Message::ServerDisconnect).await {
                    error!("rx_handler: {}", err);
                };
                break;
            }
            Ok(tungstenite::Message::Text(text)) => {
                info!("rx_handler: Recv: {}", text);

                // Deserialize received message
                match serde_json::from_str::<Message>(text.as_str()) {
                    Ok(response) => {
                        // Check if message is text message meant for chat
                        if let Message::ChatMessage(_) = response {
                            // Send to chat message receiver
                            if let Err(err) = to_chat_msg_receiver.send(Ok(response)).await {
                                error!("rx_handler: {}", err);
                            }
                        }
                        // Send message back to method
                        else if let Err(err) = to_method.send(Ok(response)).await {
                            error!("rx_handler: {}", err);
                        }
                    }
                    Err(err) => error!("rx_handler: {}", err), // Error while deserializing, no one to notify
                }
            }
            Err(err) => {
                error!("rx_handler: {}", err); // Error while reading rx_stream
                break;
            }
            _ => info!("rx_handler: Recv: Unknown message type, ignoring"), // Ignore unknown message types
        }
    }

    info!("rx_handler: Shutting down");
}

async fn tx_handler(
    mut tx_stream: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, tungstenite::Message>,
    mut from_method: Receiver<Message>,
    to_method: Sender<Result<Message, Error>>,
) {
    while let Some(message) = from_method.recv().await {
        match message {
            Message::ServerDisconnect => {
                info!("tx_handler: Server closed connection");
                break;
            }
            Message::ClientDisconnect => {
                info!("tx_handler: Client is closing connection");

                // Sending CloseFrame is handled by tungstenite
                match tx_stream.close().await {
                    Ok(_) => {
                        // Successfully closed stream, server answers with CloseFrame, notify method
                        if let Err(err) = to_method.send(Ok(Message::ClientDisconnect)).await {
                            error!("tx_handler: {}", err);
                        }
                    }
                    Err(err) => {
                        // Error while closing stream, notify method
                        if let Err(err) = to_method.send(Err(Error::Tungstenite(err))).await {
                            error!("tx_handler: {}", err);
                        }
                    }
                };
                break;
            }
            _ => {
                // Serialize message to be sent
                let json_string = serde_json::to_string(&message).unwrap();
                info!("tx_handler: Send: {}", json_string);
                let tungstenite_message = tungstenite::Message::text(json_string);

                // Sending message to server
                if let Err(err) = tx_stream.send(tungstenite_message).await {
                    if let Err(err) = to_method.send(Err(Error::Tungstenite(err))).await {
                        error!("tx_handler: {}", err); // Error while writing to tx_stream, notify method
                    }
                    break;
                };
            }
        }
    }

    info!("tx_handler: Shutting down");
}
