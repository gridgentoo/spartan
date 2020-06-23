use super::Manager;
use actix_rt::time::delay_for;
use futures_util::stream::{iter, StreamExt};
use spartan_lib::core::dispatcher::SimpleDispatcher;
use std::time::Duration;

/// Concurrently iterates over all databases in node, and executes GC on them.
async fn execute_gc(manager: &Manager<'_>) {
    iter(manager.node.db.iter())
        .for_each_concurrent(None, |(name, db)| async move {
            let mut db = db.lock().await;

            info!("Started GC cycle on database \"{}\"", name);

            db.gc();

            info!("GC cycle on \"{}\" completed successfully", name);
        })
        .await;
}

/// GC handler
///
/// Periodically iterates over all databases in node, and executes GC on them.
pub async fn spawn_gc(manager: &Manager<'_>) {
    let timer = Duration::from_secs(manager.config.gc_timer);

    loop {
        delay_for(timer).await;

        execute_gc(manager).await;
    }
}

#[cfg(test)]
mod tests {
    use super::execute_gc;
    use crate::{node::Manager, utils::testing::CONFIG};
    use spartan_lib::core::{
        dispatcher::SimpleDispatcher, message::builder::MessageBuilder, payload::Status,
    };

    #[tokio::test]
    async fn test_gc() {
        let mut manager = Manager::new(&CONFIG);

        manager.node.add("first");

        let mut message = MessageBuilder::default()
            .body("Hello, world")
            .max_tries(1)
            .compose()
            .unwrap();

        message.reserve();
        message.requeue();
        manager.queue("first").await.unwrap().push(message);

        assert_eq!(manager.queue("first").await.unwrap().size(), 1);

        execute_gc(&manager).await;

        assert_eq!(manager.queue("first").await.unwrap().size(), 0);
    }
}
