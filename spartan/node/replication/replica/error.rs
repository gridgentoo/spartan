use bincode::ErrorKind;
use thiserror::Error;
use tokio::io::Error as IoError;

#[derive(Error, Debug)]
pub enum ReplicaError {
    #[error("Unable to find replica node config")]
    ReplicaConfigNotFound,
    #[error("TCP socket error")]
    SocketError(#[from] IoError),
    #[error("Empty TCP socket")]
    EmptySocket,
    #[error("Socket codec error")]
    CodecError(#[from] Box<ErrorKind>),
    #[error("Protocol mismatch")]
    ProtocolMismatch,
}

#[cfg(test)]
impl PartialEq for ReplicaError {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (
                ReplicaError::ProtocolMismatch,
                ReplicaError::ProtocolMismatch
            )
        )
    }
}

pub type ReplicaResult<T> = Result<T, ReplicaError>;
