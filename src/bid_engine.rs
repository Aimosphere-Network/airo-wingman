use std::sync::Arc;

use thiserror::Error;
use tokio::sync::broadcast::{error::RecvError, Receiver};
use tokio_util::sync::CancellationToken;

use crate::{chain::ChainEvent, data::ModelRepo};

#[derive(Error, Debug)]
pub enum Error {
    #[error("Chain events receiver closed")]
    ReceiverClosed,
}

pub struct BidEngine {
    chain_rx: Receiver<ChainEvent>,
    model_repo: Arc<dyn ModelRepo>,
}

impl BidEngine {
    pub fn new(chain_rx: Receiver<ChainEvent>, model_repo: Arc<dyn ModelRepo>) -> Self {
        tracing::info!("🚀 Starting bid engine");
        Self { chain_rx, model_repo }
    }

    pub async fn run(&mut self, token: CancellationToken) -> crate::Result<()> {
        loop {
            tokio::select! {
                _ = token.cancelled() => break,
                result = self.chain_rx.recv() => {
                    match result {
                        Ok(event) => self.process_chain_event(event).await?,
                        Err(RecvError::Lagged(lost)) => {
                            tracing::warn!("⚠️ Chain receiver lagged behind by {lost} events");
                        },
                        Err(RecvError::Closed) => {
                            tracing::error!("Chain receiver closed");
                            return Err(Error::ReceiverClosed.into());
                        }}
                }
            }
        }
        Ok(())
    }

    async fn process_chain_event(&self, event: ChainEvent) -> crate::Result<()> {
        match event {
            ChainEvent::OrderCreated { order_id, model_id } => {
                if let Some(model) = self.model_repo.get(model_id).await {
                    tracing::info!(
                        "💸 Bidding {} on order {} for model {}",
                        model.details.bid,
                        order_id,
                        model.id
                    );
                    // TODO. Post transaction.
                }
            },
        }

        Ok(())
    }
}
