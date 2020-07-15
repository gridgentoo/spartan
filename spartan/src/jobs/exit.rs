use super::persistence::persist_manager;
use crate::node::Manager;
use actix_rt::signal::ctrl_c;
use std::process::exit;

/// Ctrl-C handler
///
/// Listens to Ctrl-C signal, and after receiving one starts persisting database.
pub async fn spawn_ctrlc_handler(manager: &Manager<'_>) {
    ctrl_c().await.expect("Unable to listen to Ctrl-C signal.");

    persist_manager(manager).await;

    exit(0);
}
