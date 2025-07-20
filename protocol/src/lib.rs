pub mod error;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    UserRegisterRequest(UserRegisterRequest),
    UserRegisterResponse(UserRegisterResponse),
    UserLoginRequest(UserLoginRequest),
    UserLoginResponse(UserLoginResponse),
    ChatMessageRequest(ChatMessageRequest),
    ChatMessageResponse(ChatMessageResponse),
    ChatMessage(ChatMessage),
    Disconnect,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserRegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserRegisterResponse {
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserLoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserLoginResponse {
    pub token: Option<String>,
    pub error: Option<String>,
}

// MessageRequest gets converted into Message on server
#[derive(Serialize, Deserialize, Debug)]
pub struct ChatMessageRequest {
    pub token: String,
    pub send_time: SystemTime,
    pub dest_user: String,
    pub content: String,
}

// As acknowledgement: (receiver) -> server -> sender
#[derive(Serialize, Deserialize, Debug)]
pub struct ChatMessageResponse {
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatMessage {
    pub src_user: String,
    pub send_time: SystemTime,
    pub content: String,
}

// Implement method so MessageRequest can be converted to Message
impl ChatMessage {
    pub fn from_request(msg_req: ChatMessageRequest, src_user: String) -> Self {
        ChatMessage {
            src_user,
            send_time: msg_req.send_time,
            content: msg_req.content,
        }
    }
}
