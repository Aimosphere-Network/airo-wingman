use async_trait::async_trait;
use std::{str::FromStr, time::Duration};
use subxt::{
    backend::{legacy::rpc_methods::Bytes, rpc::RpcClient},
    config::{
        substrate::{BlakeTwo256, SubstrateHeader},
        Hasher as HasherT, SubstrateExtrinsicParams,
    },
    custom_values::Yes,
    events::StaticEvent,
    rpc_params,
    storage::Address,
    utils::{AccountId32, MultiAddress, MultiSignature, H256},
    Config, OnlineClient,
};
use subxt_signer::{sr25519::Keypair, SecretUri};
use thiserror::Error;
use tokio::sync::broadcast::Sender;
use tokio_util::sync::CancellationToken;

use crate::types::{AgreementDetails, AgreementId, Balance, ContentId, ModelId, OrderId, Result};

#[subxt::subxt(runtime_metadata_path = "metadata.scale")]
mod airo {
    use runtime_types::pallet_execution::types::AgreementDetails as RuntimeAgreementDetails;

    use crate::types::{AgreementDetails, ModelId};

    type RuntimeModelId = runtime_types::bounded_collections::bounded_vec::BoundedVec<u8>;

    impl From<RuntimeModelId> for ModelId {
        fn from(value: RuntimeModelId) -> Self {
            String::from_utf8_lossy(&value.0).into_owned()
        }
    }

    impl From<RuntimeAgreementDetails> for AgreementDetails {
        fn from(value: RuntimeAgreementDetails) -> Self {
            Self { model_id: value.model_id.into() }
        }
    }
}

type AccountId = AccountId32;
type Block = subxt::blocks::Block<RuntimeConfig, Client>;
type Client = OnlineClient<RuntimeConfig>;
type Hasher = BlakeTwo256;

enum RuntimeConfig {}
impl Config for RuntimeConfig {
    type Hash = H256;
    type AccountId = AccountId;
    type Address = MultiAddress<Self::AccountId, ()>;
    type Signature = MultiSignature;
    type Hasher = Hasher;
    type Header = SubstrateHeader<u32, BlakeTwo256>;
    type ExtrinsicParams = SubstrateExtrinsicParams<Self>;
    type AssetId = u32;
}

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

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to get the next block")]
    NextBlock,
}

pub struct AiroClient {
    rpc: RpcClient,
    client: Client,
    signer: Keypair,
    provider: AccountId,
}

impl AiroClient {
    pub async fn new(url: &str, secret_uri: &str) -> Result<Self> {
        let uri = SecretUri::from_str(secret_uri)?;
        let signer = Keypair::from_uri(&uri)?;
        let provider = signer.public_key().to_account_id();

        // TODO. It might make sense to reconnect automatically
        // https://github.com/paritytech/subxt/blob/master/subxt/examples/setup_reconnecting_rpc_client.rs
        let rpc = RpcClient::from_insecure_url(url).await?;
        let client = Client::from_rpc_client(rpc.clone()).await?;

        tracing::info!("🚀 Connected to airo node at {url}");
        Ok(Self { rpc, client, signer, provider })
    }

    async fn fetch<'a, K, V>(&self, query: K) -> Result<Option<V>>
    where
        K: Address<IsFetchable = Yes, Target = V> + 'a,
    {
        self.client.storage().at_latest().await?.fetch(&query).await.map_err(Into::into)
    }
}

#[async_trait]
pub trait ChainListener {
    async fn listen(&self, token: CancellationToken, sender: Sender<ChainEvent>) -> Result<()>;
}

#[async_trait]
impl ChainListener for AiroClient {
    async fn listen(&self, token: CancellationToken, sender: Sender<ChainEvent>) -> Result<()> {
        // TODO. Handle `subscribe_best` might result in a possible rollback.
        let mut blocks_sub = self.client.blocks().subscribe_best().await?;
        while let Some(block) = blocks_sub.next().await {
            tokio::select! {
                _ = token.cancelled() => return Ok(()),
                result = process_block(block?, &self.provider, &sender) => result?,
            }
        }

        tracing::error!("Failed to get the next block");
        Err(Error::NextBlock.into())
    }
}

async fn process_block(
    block: Block,
    provider: &AccountId,
    sender: &Sender<ChainEvent>,
) -> Result<()> {
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
                    let model_id = event.model_id.into();
                    sender.send(ChainEvent::OrderCreated { order_id, model_id })?;
                }
            },
            (BidAccepted::PALLET, BidAccepted::EVENT) => {
                if let Some(event) = event.as_event::<BidAccepted>()? {
                    if event.provider.ne(provider) {
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

#[async_trait]
pub trait TxSubmitter {
    async fn bid_create(&self, order_id: OrderId, price_per_request: Balance) -> Result<()>;

    async fn response_create(
        &self,
        agreement_id: AgreementId,
        request_index: u32,
        content_id: ContentId,
    ) -> Result<()>;
}

#[async_trait]
impl TxSubmitter for AiroClient {
    async fn bid_create(&self, order_id: OrderId, price_per_request: Balance) -> Result<()> {
        let tx = airo::tx().airo_market().bid_create(order_id, price_per_request);
        let _hash = self.client.tx().sign_and_submit_default(&tx, &self.signer).await?;
        Ok(())
    }

    async fn response_create(
        &self,
        agreement_id: AgreementId,
        request_index: u32,
        content_id: ContentId,
    ) -> Result<()> {
        let tx =
            airo::tx()
                .airo_execution()
                .response_create(agreement_id, request_index, content_id);
        let _hash = self.client.tx().sign_and_submit_default(&tx, &self.signer).await?;
        Ok(())
    }
}

#[async_trait]
pub trait StateReader {
    async fn get_agreement(&self, agreement_id: AgreementId) -> Result<Option<AgreementDetails>>;
}

#[async_trait]
impl StateReader for AiroClient {
    async fn get_agreement(&self, agreement_id: AgreementId) -> Result<Option<AgreementDetails>> {
        let query = airo::storage().airo_execution().agreements(agreement_id);
        let agreement = self.fetch(query).await?.map(Into::into);
        Ok(agreement)
    }
}

#[async_trait]
pub trait DataExchange {
    async fn upload(&self, content_id: ContentId, data: Vec<u8>) -> Result<()>;
    async fn download(&self, key: ContentId) -> Result<Option<Vec<u8>>>;

    async fn hash_upload(&self, data: Vec<u8>) -> Result<ContentId> {
        let hash = Hasher::hash(&data);
        self.upload(hash, data).await?;
        Ok(hash)
    }

    async fn retry_download(
        &self,
        content_id: ContentId,
        max_retries: usize,
    ) -> Result<Option<Vec<u8>>> {
        let mut retries = 0;
        loop {
            match self.download(content_id).await {
                Ok(None) | Err(_) if retries < max_retries => retries += 1,
                res => return res,
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

#[async_trait]
impl DataExchange for AiroClient {
    async fn upload(&self, content_id: ContentId, data: Vec<u8>) -> Result<()> {
        let data = Bytes::from(data);
        self.rpc.request("exchange_upload", rpc_params![content_id, data]).await?;
        Ok(())
    }

    async fn download(&self, key: ContentId) -> Result<Option<Vec<u8>>> {
        let data = self.rpc.request::<Option<Bytes>>("exchange_download", rpc_params![key]).await?;
        Ok(data.map(|data| data.0))
    }
}

#[async_trait]
pub trait Protocol: TxSubmitter + StateReader + DataExchange {}

#[async_trait]
impl<T> Protocol for T where T: TxSubmitter + StateReader + DataExchange {}
