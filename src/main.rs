#![warn(missing_docs)]

use core::{
    future::Future,
    net::{Ipv4Addr, SocketAddr},
};
use std::error::Error;

use axum::Router;
use once_cell::sync::Lazy;
use tokio::{
    net::TcpListener,
    signal,
    sync::{broadcast, watch},
    task::JoinHandle,
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{config::Config};

mod check;
mod config;

static CONFIG: Lazy<Config> = Lazy::new(Config::new);

#[derive(OpenApi)]
#[openapi(paths(check::health))]
struct ApiDoc;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

async fn critical_task<F>(name: &str, token: CancellationToken, task: F) -> Result<()>
where
    F: Future<Output = Result<()>> + Send + 'static,
{
    task.await.map_err(|e| {
        tracing::error!("Critical task \"{}\" failed: {}", name, e);
        token.cancel();
        e
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry().with(tracing_subscriber::fmt::layer()).init();

    let tracker = TaskTracker::new();
    let token = CancellationToken::new();

    tracker.spawn(critical_task("http_server", token.clone(), run_http_server(token.clone())));
    tracker.spawn(shutdown_signal(token));

    tracker.close();
    tracker.wait().await;
}

async fn run_http_server(token: CancellationToken) -> Result<()> {
    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .merge(check::routes());

    let address = SocketAddr::from((Ipv4Addr::UNSPECIFIED, CONFIG.port));
    let listener = TcpListener::bind(&address).await.unwrap();

    tracing::info!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app)
        .with_graceful_shutdown(async move { token.cancelled().await })
        .await
        .map_err(Into::into)
}

async fn shutdown_signal(token: CancellationToken) {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received shutdown signal");
        },
        _ = terminate => {
            tracing::info!("Received termination signal");
        },
    }

    token.cancel();
}
