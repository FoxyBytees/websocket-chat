use persistence::crud_error::CRUDError;

#[derive(Debug)]
pub enum AccountError {
    CrudError(CRUDError),
    InvalidCredentials,
}

// Implement Display trait so error can be formatted nicely
impl std::fmt::Display for AccountError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AccountError::CrudError(err) => write!(f, "CRUD error: {}", err),
            AccountError::InvalidCredentials => write!(f, "wrong username/password"),
        }
    }
}

impl From<CRUDError> for AccountError {
    fn from(err: CRUDError) -> Self { AccountError::CrudError(err) }
}