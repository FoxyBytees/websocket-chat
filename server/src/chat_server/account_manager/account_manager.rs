use crate::chat_server::account_manager::account_error::AccountError;
use std::{
    collections::HashMap,
    sync::Arc,
};
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use protocol::Message;

pub struct Account {
    username: String,
    password: String,
}

pub struct Session {
    username: String,
    token: String,
}

pub struct AccountManager {
    accounts_by_username: Arc<RwLock<HashMap<String, Account>>>,
    senders_by_username: Arc<RwLock<HashMap<String, mpsc::Sender<Message>>>>,
    sessions_by_token: Arc<RwLock<HashMap<String, Session>>>,
}

impl AccountManager {
    pub fn new() -> Self {
        Self {
            accounts_by_username: Arc::new(RwLock::new(HashMap::new())),
            senders_by_username: Arc::new(RwLock::new(HashMap::new())),
            sessions_by_token: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn load_accounts() {}

    pub async fn create_account(
        &mut self,
        username: String,
        password: String,
    ) -> Result<(), AccountError> {
        let mut write_guard = self.accounts_by_username.write().await;

        match write_guard.insert(username.clone(), Account { username, password }) {
            Some(_) => Err(AccountError::AccountAlreadyExists),
            None => Ok(()),
        }
    }

    pub async fn remove_account(&mut self, username: String) -> Result<(), AccountError> {
        let mut write_guard = self.accounts_by_username.write().await;

        match write_guard.remove(&username) {
            Some(_) => Ok(()),
            None => Err(AccountError::AccountDoesntExist),
        }
    }

    pub async fn create_session(
        &mut self,
        username: String,
        password: String,
        sender: mpsc::Sender<Message>,
    ) -> Result<String, AccountError> {
        let read_guard = self.accounts_by_username.read().await;

        match read_guard.get(&username) {
            None => Err(AccountError::InvalidCredentials),
            Some(account) => {
                if account.password == password {
                    let token = Uuid::new_v4().to_string();
                    let mut write_guard = self.sessions_by_token.write().await;

                    match write_guard.insert(
                        token.clone(),
                        Session {
                            username: username.clone(),
                            token: token.clone(),
                        },
                    ) {
                        Some(_) => Err(AccountError::SessionAlreadyExists),
                        None => {
                            let mut write_guard = self.senders_by_username.write().await;

                            match write_guard.insert(
                                username,
                                sender
                            ) {
                                Some(_) => Err(AccountError::SessionAlreadyExists),
                                None => Ok(token),
                            }
                        }
                    }
                } else {
                    Err(AccountError::InvalidCredentials)
                }
            }
        }
    }

    pub async fn remove_session(&mut self, token: String) -> Result<(), AccountError> {
        let mut write_guard = self.sessions_by_token.write().await;

        match write_guard.remove(&token) {
            Some(session) => {
                let mut write_guard = self.senders_by_username.write().await;

                match write_guard.remove(&session.username) {
                    Some(_) => Ok(()),
                    None => Err(AccountError::SessionDoesntExist),
                }
            }
            None => Err(AccountError::SessionDoesntExist),
        }
    }

    pub async fn get_username_by_token(&self, token: &String) -> Result<String, AccountError> {
        let read_guard = self.sessions_by_token.read().await;

        match read_guard.get(token) {
            None => Err(AccountError::SessionDoesntExist),
            Some(session) => Ok(session.username.clone()),
        }
    }

    pub async fn get_sender_by_username(&self, username: &String) -> Result<mpsc::Sender<Message>, AccountError> {
        let read_guard = self.senders_by_username.read().await;

        match read_guard.get(username) {
            None => Err(AccountError::SessionDoesntExist),
            Some(sender) => Ok(sender.clone())
        }
    }
}

impl Clone for AccountManager {
    fn clone(&self) -> Self {
        Self {
            accounts_by_username: self.accounts_by_username.clone(),
            senders_by_username: self.senders_by_username.clone(),
            sessions_by_token: self.sessions_by_token.clone(),
        }
    }
}
