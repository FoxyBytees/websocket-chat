use crate::CRUD;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::hash::Hash;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::crud_error::CRUDError;

pub struct SimpleFileDB<K, V> {
    file_path: PathBuf,
    data_map: HashMap<K, V>,
}

impl<K, V> CRUD<K, V, CRUDError> for SimpleFileDB<K, V>
where K: Serialize + for<'a> Deserialize<'a> + Eq + Hash + Send + Sync,
      V: Serialize + for<'a> Deserialize<'a> + Send + Sync, {
    fn create(&mut self, key: K, value: V) -> Result<(), CRUDError> {
        match self.data_map.insert(key, value) {
            None => Ok(()),
            Some(_) => Err(CRUDError::AlreadyExists)
        }
    }

    fn read(&self, key: &K) -> Result<&V, CRUDError> {
        match self.data_map.get(key) {
            None => Err(CRUDError::NotFound),
            Some(value) => Ok(value),
        }
    }

    fn update(&mut self, key: K, value: V) -> Result<(), CRUDError> {
        match self.delete(&key) {
            Ok(_) => self.create(key, value),
            Err(err) => Err(err),
        }
    }

    fn delete(&mut self, key: &K) -> Result<V, CRUDError> {
        match self.data_map.remove(key) {
            None => Err(CRUDError::NotFound),
            Some(v) => Ok(v),
        }
    }
}

impl<K, V> SimpleFileDB<K, V>
where K: Serialize + for<'a> Deserialize<'a> + Eq + Hash,
      V: Serialize + for<'a> Deserialize<'a>, {
    pub fn new<P>(file_path: P) -> Self
    where P: AsRef<Path> {
        Self {
            file_path: file_path.as_ref().to_path_buf(),
            data_map: HashMap::<K, V>::new()
        }
    }

    pub fn load(mut self) -> Result<Self, std::io::Error> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(self.file_path.as_path())?;

        let reader = BufReader::new(file);

        for line in reader.lines() {
            match line {
                Ok(line_str) => {
                    let kv_tuple = serde_json::from_str::<(K, V)>(&line_str)?;
                    self.data_map.insert(kv_tuple.0, kv_tuple.1);
                },
                Err(err) => return Err(err),
            }
        }

        Ok(self)
    }

    pub fn save(&self) -> Result<&Self, std::io::Error> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(self.file_path.as_path())?;

        let mut writer = BufWriter::new(file);

        for line in self.data_map.iter() {
            writeln!(writer, "{}", serde_json::to_string(&(line.0, line.1))?)?;
        }

        Ok(self)
    }
}