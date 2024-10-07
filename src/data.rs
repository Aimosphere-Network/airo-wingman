use crate::types::{Hasher, Model, ModelId, ModelName};
use async_trait::async_trait;
use dashmap::DashMap;
use subxt::config::Hasher as HasherT;

#[async_trait]
pub trait ModelRepo {
    async fn list(&self) -> Vec<Model>;
    async fn contains(&self, name: &ModelName) -> bool;
    async fn get_by_model_id(&self, id: &ModelId) -> Option<Model>;
    async fn save(&self, model: Model);
    async fn remove(&self, name: &ModelName);
}

#[derive(Clone)]
pub struct InMemoryModelRepo {
    db: DashMap<ModelId, Model>,
}

#[async_trait]
impl ModelRepo for InMemoryModelRepo {
    async fn list(&self) -> Vec<Model> {
        self.db.iter().map(|kv| kv.value().clone()).collect()
    }

    async fn contains(&self, name: &ModelName) -> bool {
        let id = Hasher::hash(name.as_bytes());
        self.db.contains_key(&id)
    }

    async fn get_by_model_id(&self, id: &ModelId) -> Option<Model> {
        self.db.get(id).map(|kv| kv.value().clone())
    }

    async fn save(&self, model: Model) {
        self.db.insert(model.id, model);
    }

    async fn remove(&self, name: &ModelName) {
        let id = Hasher::hash(name.as_bytes());
        self.db.remove(&id);
    }
}

pub struct ModelRepoFac;

impl ModelRepoFac {
    pub fn in_memory() -> InMemoryModelRepo {
        InMemoryModelRepo { db: DashMap::new() }
    }
}
