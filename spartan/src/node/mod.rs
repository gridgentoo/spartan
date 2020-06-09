pub mod extractor;
pub mod gc;
pub mod manager;
pub mod persistence;

pub use extractor::QueueExtractor;
pub use manager::Manager;
pub use persistence::{load_from_fs, spawn_persistence};

use crate::server::Config;
use async_std::sync::{Mutex, MutexGuard};
use spartan_lib::core::{db::tree::TreeDatabase, message::Message};
use std::{collections::{hash_map::RandomState, HashMap}, fmt::Display};

pub type DB = TreeDatabase<Message>;
type MutexDB = Mutex<DB>;

#[derive(Default)]
pub struct Node<S = RandomState> {
    db: HashMap<String, MutexDB, S>,
}

impl Node {
    pub fn queue<T>(&self, name: T) -> Option<&MutexDB>
    where
        T: Display,
    {
        self.db.get(&name.to_string())
    }

    pub async fn get<T>(&self, name: T) -> Option<MutexGuard<'_, TreeDatabase<Message>>>
    where
        T: Display,
    {
        debug!("Obtaining queue \"{}\"", name);
        Some(self.queue(name)?.lock().await)
    }

    pub fn add<T>(&mut self, name: T)
    where
        T: Display,
    {
        info!("Initializing queue \"{}\"", name);
        self.db
            .insert(name.to_string(), Mutex::new(TreeDatabase::default()));
    }

    pub fn load_from_config(&mut self, config: &Config) {
        config.queues.iter().for_each(|queue| self.add(queue));
    }
}
