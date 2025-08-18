mod chat_server;
use crate::chat_server::chat_server::ChatServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let mut chat_server = ChatServer::new();

    chat_server.listen("0.0.0.0:8080");

    chat_server.wait_done().await?;

    Ok(())
}
