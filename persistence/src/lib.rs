extern crate core;

use std::error::Error;

pub mod crud_error;
pub mod simple_file_db;

pub trait CRUD<K, V, E>: Send + Sync
where E: Error {
    fn create(&mut self, key: K, value: V) -> Result<(), E>;
    fn read(&self, key: &K) -> Result<&V, E>;
    fn update(&mut self, key: K, value: V) -> Result<(), E>;
    fn delete(&mut self, key: &K) -> Result<V, E>;
}