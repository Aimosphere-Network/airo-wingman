use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::broadcast::{error::RecvError, Receiver};

use crate::{
    data::ModelRepo,
    engine::Engine,
    protocol::{ChainEvent, TxSubmitter},
    types::{stdResult, Result},
};

pub struct BidEngine {
    chain_rx: Receiver<ChainEvent>,
    tx_submitter: Arc<dyn TxSubmitter + Send + Sync>,
    model_repo: Arc<dyn ModelRepo + Send + Sync>,
}

impl BidEngine {
    pub fn new(
        chain_rx: Receiver<ChainEvent>,
        tx_submitter: Arc<dyn TxSubmitter + Send + Sync>,
        model_repo: Arc<dyn ModelRepo + Send + Sync>,
    ) -> Self {
        tracing::info!("🚀 Starting bid engine");
        Self { chain_rx, model_repo, tx_submitter }
    }
}

#[async_trait]
impl Engine for BidEngine {
    async fn process_chain_event(&mut self, event: ChainEvent) -> Result<()> {
        if let ChainEvent::OrderCreated { order_id, model_id } = event {
            if let Some(model) = self.model_repo.get_by_model_id(&model_id).await {
                tracing::info!(
                    "💸 Bidding {} on order {} for model {}",
                    model.details.price_per_request,
                    order_id,
                    model.id
                );

                self.tx_submitter.bid_create(order_id, model.details.price_per_request).await?;
            }
        }

        Ok(())
    }

    async fn try_recv(&mut self) -> stdResult<ChainEvent, RecvError> {
        self.chain_rx.recv().await
    }
}
