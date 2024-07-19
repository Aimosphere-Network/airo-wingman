use std::error::Error;
pub use std::result::Result as stdResult;

use primitive_types::H256;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub type Result<T> = stdResult<T, Box<dyn Error + Send + Sync>>;

pub type Balance = u128;
pub type ModelId = String;
pub type OrderId = u32;
pub type AgreementId = OrderId;
pub type ContentId = H256;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Model {
    #[schema(value_type = String)]
    pub id: ModelId,
    pub details: ModelDetails,
}

#[derive(Serialize, Deserialize, ToSchema, Clone, Copy)]
pub struct ModelDetails {
    #[schema(value_type = u128)]
    pub price_per_request: Balance,
}
