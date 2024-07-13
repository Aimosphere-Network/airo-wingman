use subxt::{
    config::{
        substrate::{BlakeTwo256, SubstrateHeader},
        SubstrateExtrinsicParams,
    },
    events::StaticEvent,
    utils::{AccountId32, MultiAddress, MultiSignature, H256},
    Config, OnlineClient,
};
use thiserror::Error;
use tokio::sync::broadcast::Sender;
use tokio_util::sync::CancellationToken;

use crate::{
    data::{ModelId, OrderId},
    Result,
};

#[subxt::subxt(runtime_metadata_path = "metadata.scale")]
pub mod airo {}

pub enum AiroConfig {}

impl Config for AiroConfig {
    type Hash = H256;
    type AccountId = AccountId32;
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
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to get the next block")]
    NextBlock,
}

pub struct ChainClient {
    pub client: AiroClient,
}

impl ChainClient {
    pub async fn new(url: &str) -> Result<Self> {
        // TODO. It might make sense to reconnect automatically
        // https://github.com/paritytech/subxt/blob/master/subxt/examples/setup_reconnecting_rpc_client.rs
        let client = AiroClient::from_insecure_url(url).await?;
        tracing::info!("🚀 Connected to airo node at {url}");
        Ok(Self { client })
    }

    pub async fn listen(&self, token: CancellationToken, sender: Sender<ChainEvent>) -> Result<()> {
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

    async fn process_block(&self, block: Block, sender: &Sender<ChainEvent>) -> Result<()> {
        let events = block.events().await?;
        for event in events.iter() {
            let event = event?;
            let event_meta = event.event_metadata();
            let pallet_name = event_meta.pallet.name();
            let event_name = event_meta.variant.name.as_str();

            if airo::airo_market::events::OrderCreated::is_event(pallet_name, event_name) {
                if let Some(order_created) =
                    event.as_event::<airo::airo_market::events::OrderCreated>()?
                {
                    let order_id = order_created.order_id;
                    let model_id = String::from_utf8_lossy(&order_created.model_id.0).into_owned();
                    sender.send(ChainEvent::OrderCreated { order_id, model_id })?;
                }
            }
        }

        Ok(())
    }
}
