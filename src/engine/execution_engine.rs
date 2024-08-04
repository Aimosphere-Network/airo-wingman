use std::{collections::HashSet, sync::Arc};

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::broadcast::{error::RecvError, Receiver};

use crate::{
    cog::{Connector, PredictionResponse},
    engine::Engine,
    protocol::{ChainEvent, Protocol},
    types::{stdResult, AgreementId, ContentId, ExecutionResult, ModelId, Result},
};

pub struct ExecutionEngine {
    chain_rx: Receiver<ChainEvent>,
    protocol_client: Arc<dyn Protocol + Send + Sync>,
    agreements: HashSet<AgreementId>,
}

impl ExecutionEngine {
    pub fn new(
        chain_rx: Receiver<ChainEvent>,
        protocol_client: Arc<dyn Protocol + Send + Sync>,
    ) -> Self {
        // TODO. Initialize agreements from the chain
        let agreements = HashSet::new();

        tracing::info!("🚀 Starting execution engine");
        Self { chain_rx, protocol_client, agreements }
    }
}

#[async_trait]
impl Engine for ExecutionEngine {
    async fn process_chain_event(&mut self, event: ChainEvent) -> Result<()> {
        match event {
            ChainEvent::BidAccepted { order_id } => {
                tracing::info!("🤝 Bid for order {order_id} accepted");
                self.agreements.insert(order_id);
            },
            ChainEvent::RequestCreated { agreement_id, request_index, content_id } => {
                if !self.agreements.contains(&agreement_id) {
                    // Skip events referencing other agreements
                    return Ok(());
                }

                process_request(
                    self.protocol_client.clone(),
                    agreement_id,
                    request_index,
                    content_id,
                )
                .await?;
            },
            _ => {},
        }

        Ok(())
    }

    async fn try_recv(&mut self) -> stdResult<ChainEvent, RecvError> {
        self.chain_rx.recv().await
    }
}

async fn process_request(
    protocol_client: Arc<dyn Protocol + Send + Sync>,
    agreement_id: AgreementId,
    request_index: u32,
    content_id: ContentId,
) -> Result<()> {
    tracing::info!("📩 Request {request_index} on agreement {agreement_id} received");
    let Some(agreement) = protocol_client.get_agreement(agreement_id).await? else {
        tracing::warn!("⚠️ Agreement {agreement_id} not found. The state might be inconsistent");
        return Ok(());
    };
    let Some(content) = protocol_client.download(content_id).await? else {
        tracing::warn!("⚠️ Content {content_id} not found");
        return Ok(());
    };
    let result = predict(&agreement.model_id, &content).await?;
    tracing::info!("🛠️ Request {request_index} on agreement {agreement_id} processed");
    let content_id = protocol_client.hash_upload(result).await?;
    protocol_client.response_create(agreement_id, request_index, content_id).await?;
    tracing::info!("✉️ Request {request_index} on agreement {agreement_id} responded");
    Ok(())
}

async fn predict(model_id: &ModelId, input: &[u8]) -> Result<Vec<u8>> {
    let input = serde_json::from_slice(input)?;
    let url = format!("http://cog-{model_id}:5000");
    tracing::debug!("🔎 Predicting {input:?} with {url}");
    let cog = Connector::new(&url)?;
    cog.ensure_ready().await?;
    let response: ExecutionResult = cog.predict::<Value, Value>(input).await?.into();
    tracing::debug!("🔎 Predicted {response:?}");
    serde_json::to_vec(&response).map_err(Into::into)
}

impl From<PredictionResponse> for ExecutionResult {
    fn from(response: PredictionResponse) -> Self {
        Self {
            status: response.status.to_string(),
            output: response.output,
            error: response.error,
            started_at: response.started_at,
            completed_at: response.completed_at,
        }
    }
}
