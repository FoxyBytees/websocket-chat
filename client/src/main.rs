mod chat_client;
use crate::chat_client::ChatClient;
use chrono::{DateTime, Local};
use colored::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // URL eines Echo-Servers, der JSON-Nachrichten zurücksendet
    let mut chat_client = ChatClient::new();

    chat_client
        .connect("ws://127.0.0.1:8080/", |chat_msg| {
            let datetime_local: DateTime<Local> = chat_msg.send_time.into();
            let formatted_local = datetime_local.format("%d.%m.%Y %H:%M:%S").to_string();

            println!(
                "[{}] {}: {}",
                formatted_local.magenta(),
                chat_msg.src_user.cyan(),
                chat_msg.content.white()
            )
        })
        .await?;

    //chat_client.register("Foxy", "TestPW").await?;

    chat_client.login("Foxy", "TestPW").await?;

    for _ in 0..5 {
        chat_client.message("Foxy", "TestMSG").await?;
    }

    chat_client.disconnect().await?;

    Ok(())
}
