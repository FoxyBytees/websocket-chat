pub mod error;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterResponse {
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginResponse {
    pub token: Option<String>,
    pub error: Option<String>,
}

// MessageRequest gets converted into Message on server
#[derive(Serialize, Deserialize, Debug)]
pub struct MessageRequest {
    pub token: String,
    pub send_time: SystemTime,
    pub dest_user: String,
    pub content: String,
}

// As acknowledgement: (receiver) -> server -> sender
#[derive(Serialize, Deserialize, Debug)]
pub struct MessageResponse {
    pub arrive_time: Option<SystemTime>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub src_user: String,
    pub send_time: SystemTime,
    pub content: String,
}

// Implement method so MessageRequest can be converted to Message
impl Message {
    pub fn from_request(msg_req: MessageRequest, src_user: String) -> Self {
        Message {
            src_user,
            send_time: msg_req.send_time,
            content: msg_req.content,
        }
    }
}
