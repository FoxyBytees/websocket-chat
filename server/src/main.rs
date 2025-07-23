mod chat_server;
use futures_util::{SinkExt, StreamExt};
use protocol::*;
use tokio::net::TcpListener;
use tokio_tungstenite::{
    accept_async,
    tungstenite::{self, Utf8Bytes},
};

use crate::chat_server::ChatServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let mut chat_server = ChatServer::new();

    chat_server.listen("0.0.0.0:8080")?;

    chat_server.wait_done().await?;

    Ok(())
}
