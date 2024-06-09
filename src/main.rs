use std::net::{Ipv4Addr, SocketAddr};

use axum::Router;
use once_cell::sync::Lazy;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::config::Config;

mod check;
mod config;

static CONFIG: Lazy<Config> = Lazy::new(Config::new);

#[derive(OpenApi)]
#[openapi(paths(check::health))]
struct ApiDoc;

#[tokio::main]
async fn main() {
    // start tracing
    tracing_subscriber::registry().with(tracing_subscriber::fmt::layer()).init();

    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .merge(check::routes());

    // start the server
    let address = SocketAddr::from((Ipv4Addr::UNSPECIFIED, CONFIG.port));
    let listener = tokio::net::TcpListener::bind(&address).await.unwrap();
    tracing::info!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
