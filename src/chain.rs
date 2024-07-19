use std::str::FromStr;

use async_trait::async_trait;
use subxt::{
    config::{
        substrate::{BlakeTwo256, SubstrateHeader},
        SubstrateExtrinsicParams,
    },
    events::StaticEvent,
    utils::{AccountId32, MultiAddress, MultiSignature, H256},
    Config, OnlineClient,
};
use subxt_signer::{sr25519::Keypair, SecretUri};
use thiserror::Error;
use tokio::sync::broadcast::Sender;
use tokio_util::sync::CancellationToken;

use crate::types::{AgreementId, Balance, ContentId, ModelId, OrderId, Result};

type AccountId = AccountId32;

#[subxt::subxt(runtime_metadata_path = "metadata.scale")]
pub mod airo {}

pub enum AiroConfig {}

impl Config for AiroConfig {
    type Hash = H256;
    type AccountId = AccountId;
    type Address = MultiAddress<Self::AccountId, ()>;
    type Signature = MultiSignature;
    type Hasher = BlakeTwo256;
    type Header = SubstrateHeader<u32, BlakeTwo256>;
    type ExtrinsicParams = SubstrateExtrinsicParams<Self>;
    type AssetId = u32;
}

type AiroClient = OnlineClient<AiroConfig>;
type Block = subxt::blocks::Block<AiroConfig, AiroClient>;

#[derive(Clone, Debug)]
pub enum ChainEvent {
    /// A new order has been created.
    OrderCreated {
        /// The order ID.
        order_id: OrderId,
        /// The model ID.
        model_id: ModelId,
    },
    /// A bid has been accepted.
    BidAccepted {
        /// The order ID.
        order_id: OrderId,
    },
    /// A request has been created.
    RequestCreated {
        /// The agreement ID.
        agreement_id: AgreementId,
        /// The request index.
        request_index: u32,
        /// The content ID.
        content_id: ContentId,
    },
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to get the next block")]
    NextBlock,
}

pub struct ChainClient {
    client: AiroClient,
    signer: Keypair,
    provider: AccountId,
}

impl ChainClient {
    pub async fn new(url: &str, secret_uri: &str) -> Result<Self> {
        let uri = SecretUri::from_str(secret_uri)?;
        let signer = Keypair::from_uri(&uri)?;
        let provider = signer.public_key().to_account_id();

        // TODO. It might make sense to reconnect automatically
        // https://github.com/paritytech/subxt/blob/master/subxt/examples/setup_reconnecting_rpc_client.rs
        let client = AiroClient::from_insecure_url(url).await?;

        tracing::info!("🚀 Connected to airo node at {url}");
        Ok(Self { client, signer, provider })
    }

    async fn process_block(&self, block: Block, sender: &Sender<ChainEvent>) -> Result<()> {
        use airo::{
            airo_execution::events::RequestCreated,
            airo_market::events::{BidAccepted, OrderCreated},
        };

        let events = block.events().await?;
        for event in events.iter() {
            let event = event?;
            let event_meta = event.event_metadata();
            let pallet_name = event_meta.pallet.name();
            let event_name = event_meta.variant.name.as_str();

            match (pallet_name, event_name) {
                (OrderCreated::PALLET, OrderCreated::EVENT) => {
                    if let Some(event) = event.as_event::<OrderCreated>()? {
                        let order_id = event.order_id;
                        let model_id = String::from_utf8_lossy(&event.model_id.0).into_owned();
                        sender.send(ChainEvent::OrderCreated { order_id, model_id })?;
                    }
                },
                (BidAccepted::PALLET, BidAccepted::EVENT) => {
                    if let Some(event) = event.as_event::<BidAccepted>()? {
                        if event.provider != self.provider {
                            // Skip events referencing other providers
                            continue;
                        }
                        let order_id = event.order_id;
                        sender.send(ChainEvent::BidAccepted { order_id })?;
                    }
                },
                (RequestCreated::PALLET, RequestCreated::EVENT) => {
                    if let Some(event) = event.as_event::<RequestCreated>()? {
                        let agreement_id = event.agreement_id;
                        let request_index = event.request_index;
                        let content_id = event.content_id;
                        sender.send(ChainEvent::RequestCreated {
                            agreement_id,
                            request_index,
                            content_id,
                        })?;
                    }
                },
                _ => {},
            }
        }
        Ok(())
    }
}

#[async_trait]
pub trait ChainListener: Send + Sync {
    async fn listen(&self, token: CancellationToken, sender: Sender<ChainEvent>) -> Result<()>;
}

#[async_trait]
impl ChainListener for ChainClient {
    async fn listen(&self, token: CancellationToken, sender: Sender<ChainEvent>) -> Result<()> {
        let mut blocks_sub = self.client.blocks().subscribe_finalized().await?;
        while let Some(block) = blocks_sub.next().await {
            tokio::select! {
                _ = token.cancelled() => return Ok(()),
                result = self.process_block(block?, &sender) => result?,
            }
        }

        tracing::error!("Failed to get the next block");
        Err(Error::NextBlock.into())
    }
}

#[async_trait]
pub trait TxSubmitter: Send + Sync {
    async fn create_bid(&self, order_id: OrderId, price_per_request: Balance) -> Result<()>;
}

#[async_trait]
impl TxSubmitter for ChainClient {
    async fn create_bid(&self, order_id: OrderId, price_per_request: Balance) -> Result<()> {
        let tx = airo::tx().airo_market().bid_create(order_id, price_per_request);
        let _hash = self.client.tx().sign_and_submit_default(&tx, &self.signer).await?;
        Ok(())
    }
}
