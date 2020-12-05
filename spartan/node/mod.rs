/// Node manager
pub mod manager;

/// Queue
pub mod queue;

/// Log-based and snapshot persistence
pub mod persistence;

/// Database event
pub mod event;

#[cfg(feature = "replication")]
/// Database replication
pub mod replication;

pub use manager::Manager;
pub use queue::Queue;

use crate::config::Config;
use spartan_lib::core::{db::TreeDatabase, message::Message};
use std::collections::{hash_map::RandomState, HashMap};

#[cfg(feature = "replication")]
use crate::node::replication::storage::ReplicationStorage;

pub type DB = Queue<TreeDatabase<Message>>;

/// Key-value node implementation
#[derive(Default)]
pub struct Node<'c, S = RandomState> {
    /// Node database
    db: HashMap<&'c str, DB, S>,
}

impl<'c> Node<'c> {
    /// Get node queue entry
    pub fn queue(&self, name: &str) -> Option<&DB> {
        self.db.get(name)
    }

    /// Add default queue entry to node
    pub fn add(&mut self, name: &'c str) {
        self.add_db(name, DB::default())
    }

    /// Add queue entry to node
    pub fn add_db(&mut self, name: &'c str, db: DB) {
        info!("Initializing queue \"{}\"", name);
        self.db.insert(name, db);
    }

    /// Iterate over queues in Node
    pub fn iter(&'c self) -> impl Iterator<Item = (&'c str, &'c DB)> {
        self.db.iter().map(|(name, db)| (*name, db))
    }

    /// Load queues from config
    pub fn load_from_config(&mut self, config: &'c Config) {
        config.queues.iter().for_each(|queue| self.add(queue));
    }

    #[cfg(feature = "replication")]
    pub async fn prepare_replication<F, R>(&self, filter: F, replace: R)
    where
        F: Fn(&ReplicationStorage) -> bool + Copy,
        R: Fn() -> ReplicationStorage + Copy,
    {
        //TODO: Concurrency
        for (_, queue) in self.iter() {
            queue.prepare_replication(filter, replace).await;
        }
    }
}
