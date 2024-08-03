use async_trait::async_trait;
use thiserror::Error;
use tokio::sync::broadcast::error::RecvError;
use tokio_util::sync::CancellationToken;

pub use bid_engine::BidEngine;
pub use execution_engine::ExecutionEngine;

use crate::{
    protocol::ChainEvent,
    types::{stdResult, Result},
};

pub mod bid_engine;
pub mod execution_engine;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Chain events receiver closed")]
    ReceiverClosed,
}

#[async_trait]
pub trait Engine {
    async fn process_chain_event(&mut self, event: ChainEvent) -> Result<()>;

    async fn try_recv(&mut self) -> stdResult<ChainEvent, RecvError>;

    async fn run(&mut self, token: CancellationToken) -> Result<()> {
        loop {
            tokio::select! {
                _ = token.cancelled() => break,
                result = self.try_recv() => {
                    match result {
                        Ok(event) => self.process_chain_event(event).await?,
                        Err(RecvError::Lagged(lost)) => {
                            tracing::warn!("⚠️ Chain receiver lagged behind by {lost} events");
                        },
                        Err(RecvError::Closed) => {
                            tracing::error!("Channel is closed");
                            return Err(Error::ReceiverClosed.into());
                        }}
                }
            }
        }
        Ok(())
    }
}
