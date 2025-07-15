use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum ErrorKind {
    ConnectionError,
    Timeout,
    RegisterError,
    LoginError,
    MessageError,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Error {
    pub kind: ErrorKind,
    pub error: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterResponse {
    pub error: Option<Error>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginResponse {
    pub token: Option<String>,
    pub error: Option<Error>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub src_user: String,
    pub dest_user: String,
    pub timestamp: u64,
    pub content: String,
}
