use crate::chat_server::account_manager::account_error::AccountError;
use std::{
    collections::HashMap,
    sync::Arc,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use persistence::CRUD;
use persistence::crud_error::CRUDError;
use protocol::Message;

#[derive(Serialize, Deserialize)]
pub struct Account {
    username: String,
    password: String,
}

pub struct Session {
    username: String,
    token: String,
}

pub struct AccountManager {
    database: Arc<RwLock<dyn CRUD<String, Account, CRUDError>>>,
    accounts_by_username: Arc<RwLock<HashMap<String, Account>>>,
    senders_by_username: Arc<RwLock<HashMap<String, mpsc::Sender<Message>>>>,
    sessions_by_token: Arc<RwLock<HashMap<String, Session>>>,
}

impl AccountManager {
    pub fn new(database: Arc<RwLock<dyn CRUD<String, Account, CRUDError>>>) -> Self {
        Self {
            database,
            accounts_by_username: Arc::new(RwLock::new(HashMap::new())),
            senders_by_username: Arc::new(RwLock::new(HashMap::new())),
            sessions_by_token: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_account(
        &mut self,
        username: String,
        password: String,
    ) -> Result<(), CRUDError> {
        let mut write_lock = self.database.write().await;
        write_lock.create(username.clone(), Account { username, password })?;
        Ok(())
    }

    pub async fn delete_account(&mut self, username: String) -> Result<(), CRUDError> {
        let mut write_lock = self.database.write().await;
        write_lock.delete(&username)?;
        Ok(())
    }

    pub async fn create_session(
        &mut self,
        username: String,
        password: String,
        sender: mpsc::Sender<Message>,
    ) -> Result<String, AccountError> {
        let read_lock = self.database.read().await;
        let account = read_lock.read(&username)?;

        if account.password == password {
            let token = Uuid::new_v4().to_string();

            match self.sessions_by_token.write().await.insert(
                token.clone(),
                Session {
                    username: username.clone(),
                    token: token.clone(),
                },
            ) {
                Some(_) => Err(AccountError::from(CRUDError::AlreadyExists)),
                None => {
                    match self.senders_by_username.write().await.insert(
                        username,
                        sender
                    ) {
                        Some(_) => Err(AccountError::from(CRUDError::AlreadyExists)),
                        None => Ok(token),
                    }
                }
            }
        } else {
            Err(AccountError::InvalidCredentials)
        }
    }

    pub async fn remove_session(&mut self, token: &String) -> Result<(), AccountError> {
        let mut write_guard = self.sessions_by_token.write().await;

        match write_guard.remove(token) {
            Some(session) => {
                let mut write_guard = self.senders_by_username.write().await;

                match write_guard.remove(&session.username) {
                    Some(_) => Ok(()),
                    None => Err(AccountError::from(CRUDError::NotFound)),
                }
            }
            None => Err(AccountError::from(CRUDError::NotFound)),
        }
    }

    pub async fn get_username_by_token(&self, token: &String) -> Result<String, AccountError> {
        let read_guard = self.sessions_by_token.read().await;

        match read_guard.get(token) {
            None => Err(AccountError::from(CRUDError::NotFound)),
            Some(session) => Ok(session.username.clone()),
        }
    }

    pub async fn get_sender_by_username(&self, username: &String) -> Result<mpsc::Sender<Message>, AccountError> {
        let read_guard = self.senders_by_username.read().await;

        match read_guard.get(username) {
            None => Err(AccountError::from(CRUDError::NotFound)),
            Some(sender) => Ok(sender.clone())
        }
    }
}

impl Clone for AccountManager {
    fn clone(&self) -> Self {
        Self {
            database: self.database.clone(),
            accounts_by_username: self.accounts_by_username.clone(),
            senders_by_username: self.senders_by_username.clone(),
            sessions_by_token: self.sessions_by_token.clone(),
        }
    }
}
