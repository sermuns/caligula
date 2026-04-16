use std::{
    collections::HashMap,
    marker::PhantomData,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    task::Poll,
};

use bincode::Options;
use bytes::Bytes;
use futures::{Sink, SinkExt, Stream, StreamExt, future::BoxFuture};
use serde::{Serialize, de::DeserializeOwned};
use tokio::sync::oneshot;
use tower::Service;
use tracing::warn;

use crate::ipcmux::{bincode_options, mux::DatagramHandler};

#[derive(Debug, thiserror::Error)]
pub enum RpcError<T> {
    #[error("Serialization failure: {0}")]
    Serialization(#[from] bincode::Error),
    #[error("Transport failure: {0}")]
    Transport(Arc<T>),
    #[error("Unexpectedly disconnected")]
    Disconnected,
}

pub fn rpc_client<Req, Res, Err>(
    rx: impl Stream<Item = Result<Bytes, Err>> + Unpin + Send + 'static,
    tx: impl Sink<Bytes, Error = Err> + Unpin + Send + 'static,
) -> impl Service<Req, Response = Res, Error = RpcError<Err>> + Send + 'static
where
    Req: Serialize + Send + 'static,
    Res: DeserializeOwned + Send + Clone + 'static,
    Err: DeserializeOwned + Send + Sync + 'static,
{
    RpcClient {
        rx,
        tx: Arc::new(tokio::sync::Mutex::new(tx)),
        map: Default::default(),
        _phantom: PhantomData,
    }
}

struct RpcClient<Rx, Tx, Req, Res, Err> {
    rx: Rx,
    tx: Arc<tokio::sync::Mutex<Tx>>,
    map: std::sync::Mutex<Option<RequestMap<Result<Res, Arc<Err>>>>>,
    _phantom: PhantomData<Req>,
}

impl<Rx, Tx, Req, Res, Err> RpcClient<Rx, Tx, Req, Res, Err>
where
    Req: Serialize + Send + 'static,
    Res: DeserializeOwned + Send + Clone + 'static,
    Err: DeserializeOwned + Send + Sync + 'static,
    Rx: Stream<Item = Result<Bytes, Err>> + Unpin + Send + 'static,
    Tx: Sink<Bytes, Error = Err> + Unpin + Send + 'static,
{
    fn handle_rx_next(&mut self, r: Option<Result<Bytes, Err>>) {
        let mut lock = self.map.lock().unwrap();
        let Some(mut map) = lock.take() else {
            return;
        };
        match r {
            Some(Ok(bs)) => {
                // Decode and send to the one request
                match bincode_options().deserialize(&bs) {
                    Ok((request_id, res)) => match map.demux_map.remove(&request_id) {
                        Some(tx) => {
                            tx.send(Ok(res)).ok();
                            *lock = Some(map);
                        }
                        None => {
                            warn!("Received response to nonexistent request {request_id}");
                            *lock = Some(map);
                        }
                    },
                    Err(e) => {
                        warn!("Failed to deserialize response from server: {e}");
                        *lock = Some(map);
                    }
                }
            }
            Some(Err(e)) => {
                // Broadcast error to everything
                map.close_with_global_result(Err(Arc::new(e)));
            }
            None => {
                // Drop the map to mark all its receivers as disconnected
                drop(map)
            }
        }
    }
}

impl<Rx, Tx, Req, Res, Err> Service<Req> for RpcClient<Rx, Tx, Req, Res, Err>
where
    Req: Serialize + Send + 'static,
    Res: DeserializeOwned + Send + Clone + 'static,
    Err: DeserializeOwned + Send + Sync + 'static,
    Rx: Stream<Item = Result<Bytes, Err>> + Unpin + Send + 'static,
    Tx: Sink<Bytes, Error = Err> + Unpin + Send + 'static,
{
    type Response = Res;

    type Error = RpcError<Err>;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        let poll = self.rx.poll_next_unpin(cx);
        match poll {
            // Still waiting on results. We can still send, so just do a no-op.
            Poll::Pending => (),

            // Result acquired
            Poll::Ready(r) => self.handle_rx_next(r),
        }

        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Req) -> Self::Future {
        // Allocate a slot in the request map
        let mut lock = self.map.lock().unwrap();
        let Some(map) = lock.as_mut() else {
            // No map means we got disconnected
            return Box::pin(std::future::ready(Err(RpcError::Disconnected)));
        };
        let (request_id, rx) = map.allocate_request_slot();
        drop(lock);

        // Attempt to serialize the request
        let bs = match bincode_options().serialize(&(request_id, req)) {
            Ok(bs) => Bytes::from(bs),
            Err(e) => return Box::pin(std::future::ready(Err(e.into()))),
        };

        // That was all relatively fast sync code! Time for the async!
        let tx = self.tx.clone();
        Box::pin(async move {
            // Actually perform the send
            let mut tx = tx.lock().await;
            tx.send(bs)
                .await
                .map_err(|e| RpcError::Transport(Arc::new(e)))?;
            drop(tx);

            // Receive the result from the map and convert it into the user type
            let rx_result: Result<Res, Arc<Err>> =
                rx.await.map_err(|_| RpcError::<Err>::Disconnected)?;
            Ok(rx_result.map_err(RpcError::Transport)?)
        })
    }
}

struct RequestMap<T> {
    next_id: u64,
    demux_map: HashMap<u64, oneshot::Sender<T>>,
}

impl<T: Clone> RequestMap<T> {
    fn new(demux_map: HashMap<u64, oneshot::Sender<T>>) -> Self {
        Self {
            next_id: 0,
            demux_map,
        }
    }

    fn allocate_request_slot(&mut self) -> (u64, oneshot::Receiver<T>) {
        let id = self.next_id;
        self.next_id += self.next_id.wrapping_add(1);

        let (tx, rx) = oneshot::channel();
        self.demux_map.insert(id, tx);

        (id, rx)
    }

    fn close_with_global_result(self, x: T) {
        for (_, tx) in self.demux_map {
            tx.send(x.clone()).ok();
        }
    }
}
