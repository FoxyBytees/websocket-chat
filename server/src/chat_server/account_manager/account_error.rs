#[derive(Debug)]
pub enum AccountError {
    AccountAlreadyExists,
    SessionAlreadyExists,
    AccountDoesntExist,
    SessionDoesntExist,
    InvalidCredentials,
}

// Implement Display trait so error can be formatted nicely
impl std::fmt::Display for AccountError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AccountError::AccountAlreadyExists => write!(f, "account already exists"),
            AccountError::SessionAlreadyExists => write!(f, "session already exists"),
            AccountError::AccountDoesntExist => write!(f, "account does not exist"),
            AccountError::SessionDoesntExist => write!(f, "session does not exist"),
            AccountError::InvalidCredentials => write!(f, "wrong username/password"),
        }
    }
}