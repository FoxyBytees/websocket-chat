use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Serialize, Deserialize, Debug, Clone)]
//#[serde(tag = "type")]
pub enum Message {
    UserRegisterRequest(UserRegisterRequest),
    UserRegisterResponse(UserRegisterResponse),
    UserLoginRequest(UserLoginRequest),
    UserLoginResponse(UserLoginResponse),
    ChatMessageRequest(ChatMessageRequest),
    ChatMessageResponse(ChatMessageResponse),
    ChatMessage(ChatMessage),
    DisconnectSuccess,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserRegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserRegisterResponse {
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserLoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserLoginResponse {
    pub session_token: Option<String>,
    pub error: Option<String>,
}

// MessageRequest gets converted into Message on server
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChatMessageRequest {
    pub session_token: String,
    pub send_time: SystemTime,
    pub dest_user: String,
    pub content: String,
}

// As acknowledgement: (receiver) -> server -> sender
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChatMessageResponse {
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

    #[cfg(test)]
    pub fn from_request_pub(msg_req: ChatMessageRequest, src_user: String) -> Self {
        Self::from_request(msg_req, src_user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[test]
    #[ignore]
    fn serde() {
        let message = Message::UserRegisterRequest(UserRegisterRequest {
            username: "Phi".to_string(),
            password: "Pass".to_string(),
        });

        let message = serde_json::to_string(&message).unwrap();

        println!("{}", message);
    }
}
