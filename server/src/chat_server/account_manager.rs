use futures_util::stream::SplitSink;
use std::{
    collections::HashMap,
    sync::{Arc, PoisonError, RwLock},
};
use tokio::{
    net::TcpStream,
    sync::{RwLockReadGuard, RwLockWriteGuard},
};
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{self},
};

struct Account {
    username: String,
    password: String,
}

struct Session {
    username: String,
    ws_tx_stream: Arc<RwLock<SplitSink<WebSocketStream<TcpStream>, tungstenite::Message>>>,
}

pub struct AccountManager {
    accounts_by_username: Arc<RwLock<HashMap<String, Account>>>,
    sessions_by_username: Arc<RwLock<HashMap<String, Session>>>,
    sessions_by_token: Arc<RwLock<HashMap<String, Session>>>,
}

impl AccountManager {
    pub fn new() -> Self {
        Self {
            accounts_by_username: Arc::new(RwLock::new(HashMap::new())),
            sessions_by_username: Arc::new(RwLock::new(HashMap::new())),
            sessions_by_token: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn load_accounts() {}

    pub fn create_account(
        &mut self,
        username: String,
        password: String,
    ) -> Result<bool, PoisonError<RwLockWriteGuard<'_, HashMap<String, Account>>>> {
        let mut write_guard = self.accounts_by_username.write()?;

        // Create new Account
        match write_guard.insert(username.clone(), Account { username, password }) {
            // Could not create account (already exists)
            Some(_) => Ok(false),
            // Successfully created account
            None => Ok(true),
        }
    }

    pub fn remove_account() {}

    pub fn create_session() {}

    pub fn remove_session() {}
}

impl Clone for AccountManager {
    fn clone(&self) -> Self {
        Self {
            accounts_by_username: self.accounts_by_username.clone(),
            sessions_by_username: self.sessions_by_username.clone(),
            sessions_by_token: self.sessions_by_token.clone(),
        }
    }
}
