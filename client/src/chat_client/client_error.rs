use protocol::Message;
use tokio_tungstenite::tungstenite;

#[derive(Debug)]
pub enum ClientError {
    IOError(std::io::Error),
    TokioMPSC(tokio::sync::mpsc::error::SendError<Message>),
    Tungstenite(tungstenite::error::Error),
    SerdeJSON(serde_json::error::Error),
    ServerError(String),
    Connected,
    Disconnected,
    Unauthenticated,
    InvalidMessage,
}

// Implement Display trait so error can be formatted nicely
impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::IOError(err) => write!(f, "IO error: {}", err),
            ClientError::TokioMPSC(err) => write!(f, "Tokio MPSC error: {}", err),
            ClientError::Tungstenite(err) => write!(f, "Tungstenite error: {}", err),
            ClientError::SerdeJSON(err) => write!(f, "Serde_JSON error: {}", err),
            ClientError::ServerError(err) => write!(f, "Server error: {}", err),
            ClientError::Connected => write!(f, "Connected: Please disconnect first"),
            ClientError::Disconnected => write!(f, "Disconnected: Please connect first"),
            ClientError::Unauthenticated => write!(f, "Unauthenticated: Please login first"),
            ClientError::InvalidMessage => {
                write!(f, "Invalid Message: This should not have happened")
            }
        }
    }
}

// Implement Error trait (common interface for all error types)
// Requires error to implement Debug and Display
// Has optional source() method, which is important for wrapping and chaining errors
impl std::error::Error for ClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            // The source of Error::WebsocketError is the underlying tungstenite::Error
            ClientError::Tungstenite(err) => Some(err),
            // Other variants don't have a direct source
            _ => None,
        }
    }
}

// Implement From trait so IO Errors can be propagated with '?'
impl From<std::io::Error> for ClientError {
    fn from(err: std::io::Error) -> Self {
        ClientError::IOError(err)
    }
}

// Implement From trait so Tokio SendErrors can be propagated with '?'
impl From<tokio::sync::mpsc::error::SendError<Message>> for ClientError {
    fn from(err: tokio::sync::mpsc::error::SendError<Message>) -> Self {
        ClientError::TokioMPSC(err)
    }
}

// Implement From trait so tungstenite Errors can be propagated with '?'
impl From<tungstenite::error::Error> for ClientError {
    fn from(err: tungstenite::error::Error) -> Self {
        ClientError::Tungstenite(err)
    }
}

// Implement From trait so serde_json Errors can be propagated with '?'
impl From<serde_json::error::Error> for ClientError {
    fn from(err: serde_json::error::Error) -> Self {
        ClientError::SerdeJSON(err)
    }
}
