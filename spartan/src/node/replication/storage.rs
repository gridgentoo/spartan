use std::collections::BTreeMap;
use super::event::Event;
use spartan_lib::core::{message::Message, dispatcher::{StatusAwareDispatcher, SimpleDispatcher, simple::{PositionBasedDelete, Delete}}, payload::Identifiable};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct ReplicationStorage {
    next_index: u64,
    gc_threshold: u64,
    log: BTreeMap<u64, Event>
}

impl Default for ReplicationStorage {
    fn default() -> Self {
        ReplicationStorage {
            next_index: 1,
            gc_threshold: 0,
            log: BTreeMap::new()
        }
    }
}

impl ReplicationStorage {
    pub fn push(&mut self, event: Event) {
        self.log.insert(self.next_index, event);
        self.next_index += 1;
    }
}

#[derive(Serialize, Deserialize)]
pub struct ReplicatedDatabase<DB> {
    inner: DB,
    storage: Option<ReplicationStorage>
}

impl<DB> Default for ReplicatedDatabase<DB>
where
    DB: Default
{
    fn default() -> Self {
        ReplicatedDatabase {
            inner: DB::default(),
            storage: None
        }
    }
}

impl<DB> ReplicatedDatabase<DB> {
    pub fn push_event<F>(&mut self, event: F)
    where
        F: FnOnce() -> Event
    {
        if let Some(storage) = &mut self.storage {
            storage.push(event());
        }
    }
}

impl<DB> SimpleDispatcher<Message> for ReplicatedDatabase<DB>
where
    DB: SimpleDispatcher<Message>,
{
    fn push(&mut self, message: Message) {
        self.push_event(|| Event::Push(message.clone()));

        self.inner.push(message)
    }

    fn peek(&self) -> Option<&Message> {
        self.inner.peek()
    }

    fn gc(&mut self) {
        self.push_event(|| Event::Gc);

        self.inner.gc()
    }

    fn size(&self) -> usize {
        self.inner.size()
    }

    fn clear(&mut self) {
        self.push_event(|| Event::Clear);

        self.inner.clear()
    }
}

impl<DB> StatusAwareDispatcher<Message> for ReplicatedDatabase<DB>
where
    DB: StatusAwareDispatcher<Message>
{
    fn pop(&mut self) -> Option<&Message> {
        self.push_event(|| Event::Pop);

        self.inner.pop()
    }

    fn requeue(&mut self, id: <Message as Identifiable>::Id) -> Option<()> {
        self.push_event(|| Event::Requeue(id));

        self.inner.requeue(id)
    }    
}

impl<DB> Delete<Message> for ReplicatedDatabase<DB>
where
    DB: Delete<Message>
{
    fn delete(&mut self, id: <Message as Identifiable>::Id) -> Option<Message> {
        self.push_event(|| Event::Delete(id));

        Delete::delete(&mut self.inner, id)
    }
}

impl<DB> PositionBasedDelete<Message> for ReplicatedDatabase<DB>
where
    DB: PositionBasedDelete<Message>
{
    fn delete(&mut self, id: <Message as Identifiable>::Id) -> Option<Message> {
        self.push_event(|| Event::Delete(id));

        PositionBasedDelete::delete(&mut self.inner, id)
    }
}
