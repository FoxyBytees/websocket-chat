#[derive(Debug)]
pub enum AccountError {
    AccountAlreadyExists,
    SessionAlreadyExists,
    AccountDoesntExist,
    SessionDoesntExist,
    InvalidCredentials,
}

// Implement Display trait so error can be formatted nicely
impl std::fmt::Display for AccountError{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AccountError::AccountAlreadyExists => write!(f, "Account already exists"),
            AccountError::SessionAlreadyExists => write!(f, "Session already exists"),
            AccountError::AccountDoesntExist => write!(f, "Account does not exist"),
            AccountError::SessionDoesntExist => write!(f, "Session does not exist"),
            AccountError::InvalidCredentials => write!(f, "Wrong username/password"),
        }
    }
}