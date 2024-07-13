use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::data::{Model, ModelDetails, ModelId, ModelRepo};

#[derive(OpenApi)]
#[openapi(
    nest(
        (path = "/v1", api = models::ModelsApi),
        (path = "/check", api = check::CheckApi),
    ),
)]
pub struct HttpServer {
    port: u16,
    model_repo: Arc<dyn ModelRepo>,
}

impl HttpServer {
    pub fn new(port: u16, model_repo: Arc<dyn ModelRepo>) -> Self {
        Self { port, model_repo }
    }
    pub async fn serve(&self, token: CancellationToken) -> crate::Result<()> {
        let app = Router::new()
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", Self::openapi()))
            .nest("/check", check::routes())
            .nest("/v1", self.v1_routes());

        let address = SocketAddr::from((Ipv4Addr::UNSPECIFIED, self.port));
        let listener = TcpListener::bind(&address).await.unwrap();

        tracing::info!("🚀 Listening on {}", listener.local_addr().unwrap());
        axum::serve(listener, app)
            .with_graceful_shutdown(async move { token.cancelled().await })
            .await
            .map_err(Into::into)
    }

    fn v1_routes(&self) -> Router {
        Router::new().merge(models::routes().with_state(models::Deps::new(self.model_repo.clone())))
    }
}

mod models {
    use super::*;

    #[derive(Clone)]
    pub struct Deps {
        model_repo: Arc<dyn ModelRepo>,
    }

    impl Deps {
        pub fn new(model_repo: Arc<dyn ModelRepo>) -> Self {
            Self { model_repo }
        }
    }

    #[derive(OpenApi)]
    #[openapi(
        paths(list_models, save_model, delete_model),
        components(schemas(Model, ModelDetails))
    )]
    pub struct ModelsApi;

    pub fn routes() -> Router<Deps> {
        Router::new()
            .route("/models", get(list_models))
            .route("/models/:id", put(save_model).delete(delete_model))
    }

    /// List all models.
    #[utoipa::path(get, path = "/models",
        responses((status = 200, description = "Ok", body = [Model])))]
    async fn list_models(State(deps): State<Deps>) -> Json<Vec<Model>> {
        let models = deps.model_repo.list().await;
        Json(models)
    }

    /// Save model. Either it's created or updated.
    #[utoipa::path(put, path = "/models/{id}",
        params(("id" = String, Path, description = "Model id")),
        request_body = ModelDetails,
        responses((status = 200, description = "Saved")))]
    async fn save_model(
        Path(id): Path<ModelId>,
        State(deps): State<Deps>,
        Json(details): Json<ModelDetails>,
    ) {
        let model = Model { id, details };
        deps.model_repo.save(model).await;
    }

    /// Delete model.
    #[utoipa::path(delete, path = "/models/{id}",
        params(("id" = String, Path, description = "Model id")),
        responses(
            (status = 200, description = "Deleted"),
            (status = 404, description = "Not found")))]
    async fn delete_model(
        Path(id): Path<ModelId>,
        State(deps): State<Deps>,
    ) -> Result<StatusCode, StatusCode> {
        if deps.model_repo.contains(&id).await {
            deps.model_repo.remove(&id).await;
            Ok(StatusCode::OK)
        } else {
            Err(StatusCode::NOT_FOUND)
        }
    }
}

mod check {
    use super::*;

    #[derive(OpenApi)]
    #[openapi(paths(health))]
    pub struct CheckApi;

    pub fn routes() -> Router {
        Router::new().route("/health", get(health))
    }

    /// Health check
    #[utoipa::path(get, path = "/health", responses((status = 200, description = "Ok")))]
    async fn health() -> String {
        "Ok".to_string()
    }
}
