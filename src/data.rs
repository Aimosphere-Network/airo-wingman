use async_trait::async_trait;
use dashmap::DashMap;

use crate::types::{Model, ModelDetails, ModelId};

#[async_trait]
pub trait ModelRepo {
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
            .map(|kv| Model { id: kv.key().clone(), details: kv.value().clone() })
            .collect()
    }

    async fn contains(&self, id: &ModelId) -> bool {
        self.db.contains_key(id)
    }

    async fn get(&self, id: ModelId) -> Option<Model> {
        self.db.get(&id).map(|kv| Model { id, details: kv.value().clone() })
    }

    async fn save(&self, model: Model) {
        self.db.insert(model.id, model.details);
    }

    async fn remove(&self, id: &ModelId) {
        self.db.remove(id);
    }
}
