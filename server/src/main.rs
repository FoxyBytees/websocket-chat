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

    // Wartet auf eingehende TCP-Verbindungen
    while let Ok((stream, _)) = listener.accept().await {
        println!("Client connected from {:?}", stream.peer_addr());

        // Starte einen neuen Tokio-Task, um die Verbindung zu verarbeiten
        tokio::spawn(handle_connection(stream));
    }

    Ok(())
}

async fn handle_connection(stream: tokio::net::TcpStream) {
    // Führe den WebSocket-Handshake durch
    let mut ws_stream = match accept_async(stream).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error during the WebSocket handshake: {}", e);
            return;
        }
    };

    // Verarbeite Nachrichten in einer Schleife
    while let Some(msg) = ws_stream.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                println!("Received message: {}", text);

                // Antwort senden
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
                // Hier könntest du die binären Daten verarbeiten
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
