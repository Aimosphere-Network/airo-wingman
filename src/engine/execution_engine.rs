use async_trait::async_trait;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::broadcast::{error::RecvError, Receiver};

use crate::{
    cog::{Connector, PredictionResponse},
    data::ModelRepo,
    engine::Engine,
    protocol::{ChainEvent, Protocol},
    retry_on_err_or_none,
    types::{stdResult, AgreementId, ContentId, ExecutionResult, ModelId, Result},
};

const FIVE_TIMES: usize = 5;

pub struct ExecutionEngine {
    chain_rx: Receiver<ChainEvent>,
    protocol_client: Arc<dyn Protocol + Send + Sync>,
    model_repo: Arc<dyn ModelRepo + Send + Sync>,
    agreements: HashMap<AgreementId, ModelId>,
}

impl ExecutionEngine {
    pub fn new(
        chain_rx: Receiver<ChainEvent>,
        protocol_client: Arc<dyn Protocol + Send + Sync>,
        model_repo: Arc<dyn ModelRepo + Send + Sync>,
    ) -> Self {
        // TODO. Initialize agreements from the chain
        let agreements = HashMap::new();

        tracing::info!("🚀 Starting execution engine");
        Self { chain_rx, protocol_client, model_repo, agreements }
    }
}

#[async_trait]
impl Engine for ExecutionEngine {
    async fn process_chain_event(&mut self, event: ChainEvent) -> Result<()> {
        match event {
            ChainEvent::BidAccepted { order_id } => {
                tracing::info!("🤝 Bid for order {order_id} accepted");
                let Some(agreement) = retry_on_err_or_none!(
                    FIVE_TIMES,
                    5000,
                    self.protocol_client.get_agreement(order_id).await
                )?
                else {
                    tracing::warn!(
                        "⚠️ Agreement {order_id} not found. The state might be inconsistent"
                    );
                    return Ok(());
                };
                self.agreements.insert(order_id, agreement.model_id);
            },
            ChainEvent::RequestCreated { agreement_id, request_index, content_id } => {
                if let Some(model_id) = self.agreements.get(&agreement_id) {
                    if let Some(model) = self.model_repo.get_by_model_id(model_id).await {
                        process_request(
                            self.protocol_client.clone(),
                            agreement_id,
                            &model.details.url,
                            request_index,
                            content_id,
                        )
                        .await?;
                    } else {
                        // Model is not served anymore
                        return Ok(());
                    }
                } else {
                    // Skip events referencing other agreements
                    return Ok(());
                }
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
    model_url: &str,
    request_index: u32,
    content_id: ContentId,
) -> Result<()> {
    tracing::info!("📩 Request {request_index} on agreement {agreement_id} received");
    let Some(content) =
        retry_on_err_or_none!(FIVE_TIMES, protocol_client.download(content_id).await)?
    else {
        tracing::warn!("⚠️ Content {content_id} not found");
        return Ok(());
    };
    let result = predict(model_url, &content).await?;
    tracing::info!("🛠️ Request {request_index} on agreement {agreement_id} processed");
    let content_id = protocol_client.hash_upload(result).await?;
    protocol_client.response_create(agreement_id, request_index, content_id).await?;
    tracing::info!("✉️ Request {request_index} on agreement {agreement_id} responded");
    Ok(())
}

async fn predict(url: &str, input: &[u8]) -> Result<Vec<u8>> {
    let input = serde_json::from_slice(input)?;
    tracing::debug!("🔎 Predicting {input:?} with {url}");
    let cog = Connector::new(url)?;
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
