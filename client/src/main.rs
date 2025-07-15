use std::time::{SystemTime, UNIX_EPOCH};
mod chat_client;

use futures_util::{SinkExt, StreamExt};
use serde_json;
use tokio::net::TcpListener;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{self, Utf8Bytes, protocol::CloseFrame},
};
use tungstenite::protocol::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // URL eines Echo-Servers, der JSON-Nachrichten zurücksendet
    let (mut ws_stream, _) = connect_async("ws://127.0.0.1:8080/").await?;
    println!("Connected!");

    let my_msg = protocol::Message {
        src_user: "Foxy".to_string(),
        dest_user: "Test".to_string(),
        content: "Hello World!".to_string(),
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
    };

    let my_msg_json = serde_json::to_string(&my_msg)?;

    println!("Sending JSON: {}", my_msg_json);
    ws_stream
        .send(Message::Text(Utf8Bytes::from(my_msg_json)))
        .await?;

    println!("Waiting for response...");
    while let Some(msg) = ws_stream.next().await {
        match msg? {
            Message::Text(text) => {
                println!("Received raw text: {}", text);
                break; // Schleife beenden nach erster Nachricht
            }
            Message::Close(_) => {
                println!("Connection closed.");
                break;
            }
            _ => (),
        }
    }

    ws_stream.close(None).await?;

    Ok(())
}
