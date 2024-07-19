use std::collections::HashSet;

use async_trait::async_trait;
use tokio::sync::broadcast::{error::RecvError, Receiver};

use crate::{chain::ChainEvent, engine::Engine, types::AgreementId};

pub struct ExecutionEngine {
    chain_rx: Receiver<ChainEvent>,
    agreements: HashSet<AgreementId>,
}

impl ExecutionEngine {
    pub fn new(chain_rx: Receiver<ChainEvent>) -> Self {
        // TODO. Initialize agreements from the chain
        let agreements = HashSet::new();

        tracing::info!("🚀 Starting execution engine");
        Self { chain_rx, agreements }
    }
}

#[async_trait]
impl Engine for ExecutionEngine {
    async fn process_chain_event(&mut self, event: ChainEvent) -> crate::Result<()> {
        match event {
            ChainEvent::BidAccepted { order_id } => {
                tracing::info!("🤝 Bid for order {order_id} accepted");
                self.agreements.insert(order_id);
            },
            _ => {},
        }

        Ok(())
    }

    async fn try_recv(&mut self) -> Result<ChainEvent, RecvError> {
        self.chain_rx.recv().await
    }
}
