use crate::core::message::Message;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BuilderError {
    #[error("no body provided for builder")]
    BodyNotProvided,
}

pub struct MessageBuilder<'a> {
    body: Option<&'a [u8]>,
    offset: i32,
    max_tries: u32,
    timeout: u32,
    delay: Option<i64>,
}

impl Default for MessageBuilder<'_> {
    fn default() -> Self {
        MessageBuilder {
            body: None,
            offset: 0,
            max_tries: 1,
            timeout: 30,
            delay: None,
        }
    }
}

impl<'a> MessageBuilder<'a> {
    pub fn body(mut self, body: &'a [u8]) -> Self {
        self.body = Some(body);
        self
    }

    pub fn offset(mut self, offset: i32) -> Self {
        self.offset = offset;
        self
    }

    pub fn max_tries(mut self, max_tries: u32) -> Self {
        self.max_tries = max_tries;
        self
    }

    pub fn timeout(mut self, timeout: u32) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn delay<F>(mut self, delay: F) -> Self
    where
        F: FnOnce(i32) -> i64,
    {
        self.delay = Some(delay(self.offset));
        self
    }

    pub fn compose(self) -> Result<Message, BuilderError> {
        if let Some(body) = self.body {
            Ok(Message::new(
                body,
                self.delay,
                self.offset,
                self.max_tries,
                self.timeout,
            ))
        } else {
            Err(BuilderError::BodyNotProvided)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MessageBuilder;
    use chrono::Utc;
    use rand::random;

    #[test]
    fn creates_message() {
        MessageBuilder::default()
            .body(&random::<[u8; 16]>())
            .max_tries(3)
            .offset(100)
            .delay(|_| Utc::now().timestamp())
            .timeout(40)
            .compose()
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn fails_with_empty_body() {
        MessageBuilder::default()
            .max_tries(3)
            .offset(100)
            .delay(|_| Utc::now().timestamp())
            .timeout(40)
            .compose()
            .unwrap();
    }
}
