mod chat_server;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::chat_server::chat_server::ChatServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        println!("Received `CTRL_CLOSE_EVENT`. Shutting down gracefully...");
        r.store(false, Ordering::SeqCst);
    })?;

    let mut chat_server = ChatServer::new();
    chat_server.listen("0.0.0.0:8081");

    loop  {
        if !running.load(Ordering::SeqCst) {
            chat_server.stop().wait_done().await?;
        break;
        }
    }

    Ok(())
}
