mod chat_server;

use crate::chat_server::chat_server::ChatServer;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    let mut chat_server = ChatServer::new();
    chat_server.listen("0.0.0.0:8081");

    tokio::signal::ctrl_c().await.expect("Failed to listen for SIGINT");
    println!("Received SIGINT: Shutting down gracefully...");

    chat_server.stop().wait_done().await?;

    Ok(())
}
