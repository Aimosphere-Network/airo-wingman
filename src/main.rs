//! TODO. Write docs.

#![warn(missing_docs)]

use axum::Router;
use core::{
    future::Future,
    net::{Ipv4Addr, SocketAddr},
};
use std::{error::Error, sync::Arc};
use tokio::{net::TcpListener, signal, sync::broadcast};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{chain::ChainClient, config::Config};

mod chain;
mod check;
mod config;

#[derive(OpenApi)]
#[openapi(paths(check::health))]
struct ApiDoc;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

/// Spawns a critical task. If the task fails, the given token is cancelled.
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
async fn main() -> Result<()> {
    let config = Config::new();
    tracing_subscriber::registry().with(tracing_subscriber::fmt::layer()).init();

    let tracker = TaskTracker::new();
    let token = CancellationToken::new();

    // A channel for receiving chain events produced by the chain listener.
    let (chain_tx, _chain_rx) = broadcast::channel(16);

    // Start the chain client.
    let chain_client = {
        let chain_client = ChainClient::new(&config.airo_node).await.map_err(|e| {
            tracing::error!("Failed to connect to airo node: {}", e);
            e
        })?;
        Arc::new(chain_client)
    };

    // Spawn the chain listener.
    {
        let chain_client = chain_client.clone();
        let token = token.clone();
        tracker.spawn(critical_task("chain_listener", token.clone(), async move {
            chain_client.listen(token, chain_tx).await
        }));
    }

    // Spawn the HTTP server.
    tracker.spawn(critical_task(
        "http_server",
        token.clone(),
        http_server(token.clone(), config.port),
    ));

    tracker.spawn(shutdown_signal(token));

    tracker.close();
    tracker.wait().await;
    Ok(())
}

async fn http_server(token: CancellationToken, port: u16) -> Result<()> {
    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .merge(check::routes());

    let address = SocketAddr::from((Ipv4Addr::UNSPECIFIED, port));
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
            token.cancel();
        },
        _ = terminate => {
            tracing::info!("Received termination signal");
            token.cancel();
        },
        _ = token.cancelled() => {},
    }
}
