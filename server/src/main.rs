use futures_util::{SinkExt, StreamExt};
use protocol::*;
use tokio::net::TcpListener;
use tokio_tungstenite::{
    accept_async,
    tungstenite::{self, Utf8Bytes},
};
use tungstenite::protocol::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    while let Some(msg) = ws_stream.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                println!("Received message: {}", text);

                // Send response
                let login_request = LoginResponse {
                    token: Some(String::from("TestTKN")),
                    error: None,
                };
                let json = serde_json::to_string(&login_request).unwrap();

                if let Err(e) = ws_stream.send(Message::Text(Utf8Bytes::from(json))).await {
                    eprintln!("Error sending message: {}", e);
                    break;
                }
            }
            Ok(Message::Binary(bin)) => {
                println!("Received binary data of length: {}", bin.len());
            }
            Ok(Message::Close(_)) => {
                println!("Client requested to close the connection.");
                break;
            }
            Err(e) => {
                eprintln!("Error reading message: {}", e);
                break;
            }
            _ => {} // Andere Nachrichtentypen ignorieren
        }
    }
}
