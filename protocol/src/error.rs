#[derive(Debug)]
pub enum Error {
    Tungstenite(tungstenite::Error),
    SerdeJSON(serde_json::Error),
    Disconnected(String),
    RegisterFailed(String),
    LoginFailed(String),
    MessageFailed(String),
}

// Implement Display trait so error can be formatted nicely
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Tungstenite(err) => write!(f, "Tungstenite error: {}", err),
            Error::SerdeJSON(err) => write!(f, "Serde_JSON error: {}", err),
            Error::Disconnected(desc) => write!(f, "Disconnected: {}", desc),
            Error::RegisterFailed(desc) => write!(f, "Error while registering: {}", desc),
            Error::LoginFailed(desc) => write!(f, "Error while logging in: {}", desc),
            Error::MessageFailed(desc) => write!(f, "Error while sending message: {}", desc),
        }
    }
}

// Implement Error trait (common interface for all error types)
// Requires error to implement Debug and Display
// Has optional source() method, which is important for wrapping and chaining errors
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            // The source of Error::WebsocketError is the underlying tungstenite::Error
            Error::Tungstenite(err) => Some(err),
            // Other variants don't have a direct source
            _ => None,
        }
    }
}

// Implement From trait so tungstenite Errors can be propagated with '?'
impl From<tungstenite::Error> for Error {
    fn from(err: tungstenite::Error) -> Self {
        Error::Tungstenite(err)
    }
}

// Implement From trait so serde_json Errors can be propagated with '?'
impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::SerdeJSON(err)
    }
}
