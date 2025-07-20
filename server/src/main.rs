mod chat_server;
use futures_util::{SinkExt, StreamExt};
use protocol::*;
use tokio::net::TcpListener;
use tokio_tungstenite::{
    accept_async,
    tungstenite::{self, Utf8Bytes},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(&addr).await?;

    // Wait for incoming TCP connection
    while let Ok((stream, _)) = listener.accept().await {
        println!("Client connected from {:?}", stream.peer_addr());

        // Start new task for handling connection
        tokio::spawn(handle_connection(stream));
    }

    Ok(())
}

async fn handle_connection(stream: tokio::net::TcpStream) {
    // Do websocket handshake
    let mut ws_stream = match accept_async(stream).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error during the WebSocket handshake: {}", e);
            return;
        }
    };

    // Process data in loop
    while let Some(rx_result) = ws_stream.next().await {
        match rx_result {
            Ok(tungstenite::Message::Text(text)) => {
                println!("handle_connection: Recv: {}", text);

                // Deserialize received message
                match serde_json::from_str::<Message>(text.as_str()) {
                    Ok(message) => match message {
                        Message::ChatMessageRequest(msg_req) => {
                            let response =
                                Message::ChatMessageResponse(ChatMessageResponse { error: None });

                            // Send response
                            if let Err(e) = ws_stream
                                .send(tungstenite::Message::Text(Utf8Bytes::from(
                                    serde_json::to_string(&response).unwrap(),
                                )))
                                .await
                            {
                                eprintln!("handle_connection: {}", e);
                                break;
                            }

                            let chat_msg = Message::ChatMessage(ChatMessage::from_request(
                                msg_req,
                                String::from("Server"),
                            ));

                            // Send response
                            if let Err(e) = ws_stream
                                .send(tungstenite::Message::Text(Utf8Bytes::from(
                                    serde_json::to_string(&chat_msg).unwrap(),
                                )))
                                .await
                            {
                                eprintln!("handle_connection: {}", e);
                                break;
                            }

                            //ws_stream.close(None).await;
                        }
                        Message::UserLoginRequest(_) => {
                            let response = Message::UserLoginResponse(UserLoginResponse {
                                token: Some(String::from("TestTKN")),
                                error: None,
                            });

                            // Send response
                            if let Err(e) = ws_stream
                                .send(tungstenite::Message::Text(Utf8Bytes::from(
                                    serde_json::to_string(&response).unwrap(),
                                )))
                                .await
                            {
                                eprintln!("handle_connection: {}", e);
                                break;
                            }
                        }
                        _ => {}
                    },
                    Err(err) => eprintln!("handle_connection: {}", err),
                }
            }
            Ok(tungstenite::Message::Binary(bin)) => {
                println!("handle_connection: Recv {} Byte", bin.len());
            }
            Ok(tungstenite::Message::Close(_)) => {
                println!("handle_connection: Client closed connection");
                break;
            }
            Err(e) => {
                eprintln!("handle_connection: {}", e);
                break;
            }
            _ => {} // Ignore other message types
        }
    }

    println!("handle_connection: Shutting down");
}
