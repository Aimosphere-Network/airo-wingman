use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub type Balance = u128;
pub type ModelId = String;
pub type OrderId = u32;

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

#[async_trait]
pub trait ModelRepo: Send + Sync {
    async fn list(&self) -> Vec<Model>;
    async fn contains(&self, id: &ModelId) -> bool;
    async fn get(&self, id: ModelId) -> Option<Model>;
    async fn save(&self, model: Model);
    async fn remove(&self, id: &ModelId);
}

#[derive(Clone)]
pub struct InMemoryModelRepo {
    db: DashMap<ModelId, ModelDetails>,
}

pub struct ModelRepoFac;

impl ModelRepoFac {
    pub fn in_memory() -> InMemoryModelRepo {
        InMemoryModelRepo { db: DashMap::new() }
    }
}

#[async_trait]
impl ModelRepo for InMemoryModelRepo {
    async fn list(&self) -> Vec<Model> {
        self.db
            .iter()
            .map(|kv| Model { id: kv.key().clone(), details: *kv.value() })
            .collect()
    }

    async fn contains(&self, id: &ModelId) -> bool {
        self.db.contains_key(id)
    }

    async fn get(&self, id: ModelId) -> Option<Model> {
        self.db.get(&id).map(|kv| Model { id, details: *kv.value() })
    }

    async fn save(&self, model: Model) {
        self.db.insert(model.id, model.details);
    }

    async fn remove(&self, id: &ModelId) {
        self.db.remove(id);
    }
}
