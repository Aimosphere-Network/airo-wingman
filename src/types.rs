use std::error::Error;
pub use std::result::Result as stdResult;

use primitive_types::H256;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use subxt::config::{substrate::BlakeTwo256, Hasher as HasherT};
use utoipa::ToSchema;

pub type Result<T> = stdResult<T, Box<dyn Error + Send + Sync>>;

pub type Hasher = BlakeTwo256;

pub type Balance = u128;
pub type ModelName = String;
pub type ModelId = H256;
pub type OrderId = u32;
pub type ContentId = H256;
pub type AgreementId = OrderId;
pub struct AgreementDetails {
    pub model_id: ModelId,
}

#[derive(Debug, Serialize)]
pub struct ExecutionResult {
    pub status: String,
    pub output: Option<Value>,
    pub error: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct Model {
    #[schema(value_type = [u8; 32])]
    pub id: ModelId,
    #[schema(value_type = String)]
    pub name: ModelName,
    pub details: ModelDetails,
}

impl Model {
    pub fn new(name: ModelName, details: ModelDetails) -> Self {
        let id = Hasher::hash(name.as_bytes());
        Self { id, name, details }
    }
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct ModelDetails {
    #[schema(value_type = u128)]
    pub price_per_request: Balance,
    pub url: String,
}
