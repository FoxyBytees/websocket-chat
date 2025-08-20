mod chat_client;
use crate::chat_client::chat_client::ChatClient;
use chrono::{DateTime, Local};
use colored::*;
use std::{env};
use std::io::{stdout, stdin, Write};
use std::sync::Arc;
use std::sync::RwLock;
use protocol::ChatMessage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let server_addr = args[1].clone();
    let from_user = args[2].clone();
    let from_user_clone = from_user.clone();
    let to_user = args[3].clone();
    let msg_list = Arc::new(RwLock::new(Vec::<ChatMessage>::new()));

    let mut chat_client = ChatClient::new();
    chat_client.connect(format!("ws://{}/", server_addr), move |chat_msg| {
        print!("\x1B[2J\x1B[1;1H");

        if chat_msg.src_user.trim() != from_user_clone {
            print!("\x07");
        }

        msg_list.write().unwrap().push(chat_msg);

        for chat_msg in msg_list.read().unwrap().iter() {
            let datetime_local: DateTime<Local> = chat_msg.send_time.into();
            let formatted_local = datetime_local.format("%d.%m.%Y %H:%M:%S").to_string();

            if chat_msg.src_user.trim() != from_user_clone {
                print!(
                    "[{}] {}: {}",
                    formatted_local.magenta(),
                    chat_msg.src_user.cyan(),
                    chat_msg.content.white()
                );
            }
            else {
                print!(
                    "[{}] {}: {}",
                    formatted_local.magenta(),
                    chat_msg.src_user.green(),
                    chat_msg.content.white()
                );
            }
        }

        print!("{}", String::from("Message: ").green());
        stdout().flush().unwrap();

    }).await?;

    chat_client.register(&*from_user, "TestPW").await?;
    chat_client.login(&*from_user, "TestPW").await?;

    loop {
        let mut message= String::new();

        stdin().read_line(&mut message).expect("Failed to read line");

        if message.trim() == "exit()" {
            chat_client.disconnect().await?.wait_done().await?;
            break;
        }

        chat_client.message(&*to_user, &*message).await?;
    }

    Ok(())
}
