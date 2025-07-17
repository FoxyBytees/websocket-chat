mod chat_client;
use crate::chat_client::ChatClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // URL eines Echo-Servers, der JSON-Nachrichten zurücksendet
    let mut chat_client = ChatClient::new();

    chat_client.connect("ws://127.0.0.1:8080/").await?;

    //chat_client.register("Foxy", "TestPW").await?;

    chat_client.login("Foxy", "TestPW").await?;

    //chat_client.message("Foxy", "TestMSG").await?;

    chat_client.disconnect().await?;

    Ok(())
}
