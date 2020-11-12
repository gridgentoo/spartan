use super::{
    error::{PrimaryError, PrimaryResult},
    index::{BatchAskIndex, RecvIndex},
};
use crate::{
    config::replication::Primary,
    node::{
        event::Event,
        replication::message::{PrimaryRequest, ReplicaRequest, Request},
    },
    utils::codec::BincodeCodec,
};
use futures_util::{stream::iter, SinkExt, StreamExt, TryStreamExt};
use maybe_owned::MaybeOwned;
use std::borrow::Cow;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use tokio_util::codec::{Decoder, Framed};

pub struct Stream<T>(Framed<T, BincodeCodec>);

pub struct StreamPool<T>(Box<[Stream<T>]>);

impl<'a, T> Stream<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn exchange(
        &mut self,
        message: PrimaryRequest<'_>,
    ) -> PrimaryResult<ReplicaRequest<'_>> {
        self.0
            .send(Request::Primary(message))
            .await
            .map_err(PrimaryError::CodecError)?;

        SinkExt::<Request>::flush(&mut self.0)
            .await
            .map_err(PrimaryError::CodecError)?;

        let buf = match self.0.next().await {
            Some(r) => r.map_err(PrimaryError::CodecError)?,
            None => return Err(PrimaryError::EmptySocket),
        };

        buf.get_replica().ok_or(PrimaryError::ProtocolMismatch)
    }

    async fn ping(&mut self) -> PrimaryResult<()> {
        match self.exchange(PrimaryRequest::Ping).await? {
            ReplicaRequest::Pong => Ok(()),
            _ => Err(PrimaryError::ProtocolMismatch),
        }
    }

    async fn ask(&'a mut self) -> PrimaryResult<RecvIndex<'a, T>> {
        match self.exchange(PrimaryRequest::AskIndex).await? {
            ReplicaRequest::RecvIndex(recv) => Ok(RecvIndex::new(self, recv)),
            _ => Err(PrimaryError::ProtocolMismatch),
        }
    }

    pub(super) async fn send_range(
        &mut self,
        queue: &str,
        range: Box<[(MaybeOwned<'a, u64>, MaybeOwned<'a, Event<'static>>)]>,
    ) -> PrimaryResult<()> {
        match self
            .exchange(PrimaryRequest::SendRange(Cow::Borrowed(queue), range))
            .await?
        {
            ReplicaRequest::RecvRange => Ok(()),
            ReplicaRequest::QueueNotFound(queue) => {
                warn!("Queue {} not found on replica", queue);
                Ok(())
            }
            _ => Err(PrimaryError::ProtocolMismatch),
        }
    }
}

impl<'a> StreamPool<TcpStream> {
    pub async fn from_config(config: &Primary) -> PrimaryResult<Self> {
        let mut pool = Vec::with_capacity(config.destination.len());

        for host in &*config.destination {
            pool.push(
                TcpStream::connect(host)
                    .await
                    .map_err(PrimaryError::SocketError)?,
            );
        }

        Ok(StreamPool::new(pool).await)
    }
}

impl<'a, T> StreamPool<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn new<P>(pool: P) -> Self
    where
        P: IntoIterator<Item = T>,
    {
        let pool = pool
            .into_iter()
            .map(|stream| Stream(BincodeCodec::default().framed(stream)))
            .collect::<Vec<_>>();

        StreamPool(pool.into_boxed_slice())
    }

    pub async fn ping(&mut self) -> PrimaryResult<()> {
        debug!("Pinging stream pool.");

        iter(self.0.iter_mut())
            .map(Ok)
            .try_for_each_concurrent(None, |stream| async move { stream.ping().await })
            .await?;

        Ok(())
    }

    pub async fn ask(&'a mut self) -> PrimaryResult<BatchAskIndex<'a, T>> {
        let len = self.0.len();

        debug!("Asking stream pool of {} nodes for indexes.", len);

        let mut batch = BatchAskIndex::with_capacity(len);

        for host in &mut *self.0 {
            batch.push(host.ask().await?);
        }

        Ok(batch)
    }
}

#[cfg(test)]
mod tests {
    use super::StreamPool;
    use crate::{
        node::replication::message::{PrimaryRequest, ReplicaRequest, Request},
        utils::{codec::BincodeCodec, stream::TestStream},
    };
    use actix_web::web::BytesMut;
    use bincode::deserialize;

    #[tokio::test]
    async fn test_ping() {
        let mut buf = BytesMut::default();
        let stream = vec![TestStream::from_output(
            Request::Replica(ReplicaRequest::Pong),
            &mut BincodeCodec,
        )
        .unwrap()
        .input(&mut buf)];

        StreamPool::new(stream).await.ping().await.unwrap();
        assert_eq!(
            deserialize::<Request>(&*buf).unwrap(),
            Request::Primary(PrimaryRequest::Ping)
        );
    }

    #[tokio::test]
    async fn test_ask() {
        let mut buf = BytesMut::default();
        let stream = vec![TestStream::from_output(
            Request::Replica(ReplicaRequest::RecvIndex(
                vec![(String::from("test").into_boxed_str(), 123)].into_boxed_slice(),
            )),
            &mut BincodeCodec,
        )
        .unwrap()
        .input(&mut buf)];

        StreamPool::new(stream).await.ask().await.unwrap();
        assert_eq!(
            deserialize::<Request>(&*buf).unwrap(),
            Request::Primary(PrimaryRequest::AskIndex)
        );
    }

    // TODO: TestStream with multiple input and output buffers
    // #[tokio::test]
    // async fn test_send_range() {
    //     let mut buf = BytesMut::default();
    //     let stream = vec![TestStream::from_output(
    //         Request::Replica(ReplicaRequest::RecvIndex(
    //             vec![(String::from("test").into_boxed_str(), 0)].into_boxed_slice(),
    //         )),
    //         &mut BincodeCodec,
    //     )
    //     .unwrap()
    //     .input(&mut buf)];

    //     let manager = Manager::new(&CONFIG);
    //     manager
    //         .queue("test")
    //         .unwrap()
    //         .prepare_replication(
    //             |_| false,
    //             || ReplicationStorage::Primary(PrimaryStorage::default()),
    //         )
    //         .await;
    //     manager.queue("test").unwrap().database().await.gc();

    //     StreamPool::new(stream)
    //         .await
    //         .ask()
    //         .await
    //         .unwrap()
    //         .sync(&manager)
    //         .await
    //         .unwrap();
    // }
}
