mod chat_server;

use crate::chat_server::chat_server::ChatServer;
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;
use persistence::simple_file_db::simple_file_db::SimpleFileDB;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let server_addr = args[1].clone();

    let db_arc = Arc::new(RwLock::new(
        SimpleFileDB::new("accounts.sdb").load()?
    ));
    let mut chat_server = ChatServer::new(db_arc.clone());
    chat_server.listen(format!("{}", server_addr));

    tokio::signal::ctrl_c().await.expect("failed to listen for SIGINT");
    println!("received SIGINT: shutting down gracefully...");

    db_arc.write().await.save()?;
    chat_server.stop().wait_done().await?;

    Ok(())
}
