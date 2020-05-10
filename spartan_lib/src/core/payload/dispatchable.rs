use super::Identifiable;

/// Interface for working with dispatchable messages
pub trait Dispatchable: Identifiable {
    /// Check if current message is obtainable
    ///
    /// ```
    /// use spartan_lib::core::message::builder::MessageBuilder;
    /// use spartan_lib::core::payload::Dispatchable;
    /// use spartan_lib::chrono::{Utc, Duration};
    ///
    /// let message = MessageBuilder::default().body(b"Hello, world").compose().unwrap();
    /// let delayed_message = MessageBuilder::default()
    ///     .body(b"Hello, world")
    ///     .delay(|tz| (Utc::now() + Duration::minutes(10)).timestamp())
    ///     .compose()
    ///     .unwrap();
    ///
    /// assert!(message.obtainable());
    /// assert!(!delayed_message.obtainable());
    /// ```
    fn obtainable(&self) -> bool;

    /// Check if current message is garbage
    ///
    /// ```
    /// use spartan_lib::core::message::builder::MessageBuilder;
    /// use spartan_lib::core::payload::{Dispatchable, Status};
    /// use std::thread::sleep;
    /// use std::time::Duration;
    ///
    /// let mut message = MessageBuilder::default()
    ///     .body(b"Hello, world")
    ///     .timeout(0)
    ///     .compose()
    ///     .unwrap();
    ///
    /// message.reserve();
    ///
    /// sleep(Duration::from_secs(2));
    ///
    /// assert!(message.gc());
    /// ```
    fn gc(&self) -> bool;
}
