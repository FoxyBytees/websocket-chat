mod chat_client;

use crate::chat_client::chat_client::ChatClient;
use chrono::{DateTime, Local};
use colored::*;
use std::{env};
use std::io::{stdout, stdin, Write};
use std::sync::{Arc,RwLock};
use protocol::ChatMessage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let server_addr = args[1].clone();
    let username = Arc::new(RwLock::new(String::new()));
    let username_clone = username.clone();
    let msg_list = Arc::new(RwLock::new(Vec::<ChatMessage>::new()));

    let mut chat_client = ChatClient::new();
    chat_client.connect(
        format!("ws://{}/", server_addr),
        move |chat_msg| {
        print!("\x1B[2J\x1B[1;1H");
        let username = username_clone.read().unwrap().trim().to_owned();

        if chat_msg.src_user.trim() != username {
            print!("\x07");
        }

        msg_list.write().unwrap().push(chat_msg);

        for chat_msg in msg_list.read().unwrap().iter() {
            let datetime_local: DateTime<Local> = chat_msg.send_time.into();
            let formatted_local = datetime_local.format("%d.%m.%Y %H:%M:%S").to_string();

            if chat_msg.src_user.trim() != username {
                println!(
                    "[{}] {}: {}",
                    formatted_local.magenta(),
                    chat_msg.src_user.cyan(),
                    chat_msg.content.white()
                );
            }
            else {
                println!(
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

    loop{
        print!("\x1B[2J\x1B[1;1H");
        println!("1: Login");
        println!("2: Register");
        print!("Choice: ");
        let mut choice = String::new();
        stdout().flush().unwrap();
        stdin().read_line(&mut choice)?;

        print!("\x1B[2J\x1B[1;1H");
        print!("Username: ");
        stdout().flush().unwrap();
        stdin().read_line(&mut username.write().unwrap())?;

        print!("\x1B[2J\x1B[1;1H");
        print!("Password: ");
        let mut password = String::new();
        stdout().flush().unwrap();
        stdin().read_line(&mut password)?;

        match choice.trim() {
            "1" => {
                chat_client.login(username.read().unwrap().trim(), password.trim()).await?;
                break;
            }
            "2" => {
                chat_client.register(username.read().unwrap().trim(), password.trim()).await?;
            }
            _ => {}
        }
    }

    print!("\x1B[2J\x1B[1;1H");
    print!("User to message: ");
    let mut to_user = String::new();
    stdout().flush().unwrap();
    stdin().read_line(&mut to_user)?;

    loop {
        let mut message= String::new();

        stdin().read_line(&mut message)?;

        if message.trim() == "exit()" {
            chat_client.disconnect().await?.wait_done().await?;
            break;
        }

        chat_client.message(to_user.trim(), message.trim()).await?;
    }

    Ok(())
}
