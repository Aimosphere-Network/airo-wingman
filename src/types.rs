use std::error::Error;
pub use std::result::Result as stdResult;

use primitive_types::H256;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

pub type Result<T> = stdResult<T, Box<dyn Error + Send + Sync>>;

pub type Balance = u128;
pub type ModelId = String;
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

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Model {
    #[schema(value_type = String)]
    pub id: ModelId,
    pub details: ModelDetails,
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct ModelDetails {
    #[schema(value_type = u128)]
    pub price_per_request: Balance,
}
