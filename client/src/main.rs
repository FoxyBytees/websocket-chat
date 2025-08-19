mod chat_client;
use crate::chat_client::chat_client::ChatClient;
use chrono::{DateTime, Local};
use colored::*;
use std::{env, io};
use std::io::Write;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args: Vec<String> = env::args().collect();
    let from_user = args[1].clone();
    let to_user = args[2].clone();
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

    chat_client.register(&*from_user, "TestPW").await?;

    chat_client.login(&*from_user, "TestPW").await?;

    loop {
        let mut message= String::new();

        print!("Message: ");
        io::stdout().flush()?;
        io::stdin()
            .read_line(&mut message)
            .expect("Failed to read line");

        if message == "exit" {
            break;
        }

        chat_client.message(&*to_user, &*message).await?;
    }

    chat_client.disconnect().await?.wait_done().await?;

    Ok(())
}
