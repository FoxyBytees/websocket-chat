use crate::Message;

#[derive(Debug)]
pub enum Error {
    TokioMPSC(tokio::sync::mpsc::error::SendError<Message>),
    Tungstenite(tungstenite::error::Error),
    SerdeJSON(serde_json::error::Error),
    ServerError(String),
    Disconnected,
    Unauthenticated,
    InvalidMessage,
}

// Implement Display trait so error can be formatted nicely
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::TokioMPSC(err) => write!(f, "Tokio MPSC error: {}", err),
            Error::Tungstenite(err) => write!(f, "Tungstenite error: {}", err),
            Error::SerdeJSON(err) => write!(f, "Serde_JSON error: {}", err),
            Error::ServerError(err) => write!(f, "Internal Server Error: {}", err),
            Error::Disconnected => write!(f, "Disconnected: Please connect first"),
            Error::Unauthenticated => write!(f, "Unauthenticated: Please login first"),
            Error::InvalidMessage => write!(f, "Invalid Message: This should not have happened"),
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

// Implement From trait so Tokio SendErrors can be propagated with '?'
impl From<tokio::sync::mpsc::error::SendError<Message>> for Error {
    fn from(err: tokio::sync::mpsc::error::SendError<Message>) -> Self {
        Error::TokioMPSC(err)
    }
}

// Implement From trait so tungstenite Errors can be propagated with '?'
impl From<tungstenite::error::Error> for Error {
    fn from(err: tungstenite::error::Error) -> Self {
        Error::Tungstenite(err)
    }
}

// Implement From trait so serde_json Errors can be propagated with '?'
impl From<serde_json::error::Error> for Error {
    fn from(err: serde_json::error::Error) -> Self {
        Error::SerdeJSON(err)
    }
}
