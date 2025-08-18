use crate::chat_server::account_manager::account_manager::{Account, Session};
use std::collections::HashMap;
use std::sync::PoisonError;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

#[derive(Debug)]
pub enum AccountError<'a> {
    AccountWriteError(PoisonError<RwLockWriteGuard<'a, HashMap<String, Account>>>),
    SessionWriteError(PoisonError<RwLockWriteGuard<'a, HashMap<String, Session>>>),
    AccountReadError(PoisonError<RwLockReadGuard<'a, HashMap<String, Account>>>),
    SessionReadError(PoisonError<RwLockReadGuard<'a, HashMap<String, Session>>>),
    AccountAlreadyExists,
    SessionAlreadyExists,
    AccountDoesntExist,
    SessionDoesntExist,
    InvalidCredentials,
}

// Implement Display trait so error can be formatted nicely
impl std::fmt::Display for AccountError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AccountError::AccountWriteError(err) => write!(f, "Account write error: {}", err),
            AccountError::SessionWriteError(err) => write!(f, "Session write error: {}", err),
            AccountError::AccountReadError(err) => write!(f, "Account read error: {}", err),
            AccountError::SessionReadError(err) => write!(f, "Session read error: {}", err),
            AccountError::AccountAlreadyExists => write!(f, "Account already exists"),
            AccountError::SessionAlreadyExists => write!(f, "Session already exists"),
            AccountError::AccountDoesntExist => write!(f, "Account does not exist"),
            AccountError::SessionDoesntExist => write!(f, "Session does not exist"),
            AccountError::InvalidCredentials => write!(f, "Wrong username/password"),
        }
    }
}

// Implement Error trait (common interface for all error types)
// Requires error to implement Debug and Display
// Has optional source() method, which is important for wrapping and chaining errors
impl std::error::Error for AccountError<'static> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AccountError::AccountWriteError(err) => Some(err),
            AccountError::SessionWriteError(err) => Some(err),
            AccountError::AccountReadError(err) => Some(err),
            AccountError::SessionReadError(err) => Some(err),
            // Other variants don't have a direct source
            _ => None,
        }
    }
}

// Implement From trait so PoisonErrors can be propagated with '?'
impl<'a> From<PoisonError<RwLockWriteGuard<'a, HashMap<String, Account>>>> for AccountError<'a> {
    fn from(err: PoisonError<RwLockWriteGuard<'a, HashMap<String, Account>>>) -> Self {
        AccountError::AccountWriteError(err)
    }
}

impl<'a> From<PoisonError<RwLockWriteGuard<'a, HashMap<String, Session>>>> for AccountError<'a> {
    fn from(err: PoisonError<RwLockWriteGuard<'a, HashMap<String, Session>>>) -> Self {
        AccountError::SessionWriteError(err)
    }
}

impl<'a> From<PoisonError<RwLockReadGuard<'a, HashMap<String, Account>>>> for AccountError<'a> {
    fn from(err: PoisonError<RwLockReadGuard<'a, HashMap<String, Account>>>) -> Self {
        AccountError::AccountReadError(err)
    }
}

impl<'a> From<PoisonError<RwLockReadGuard<'a, HashMap<String, Session>>>> for AccountError<'a> {
    fn from(err: PoisonError<RwLockReadGuard<'a, HashMap<String, Session>>>) -> Self {
        AccountError::SessionReadError(err)
    }
}
