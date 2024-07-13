//! TODO. Write docs.

#![warn(missing_docs)]

use core::future::Future;
use std::{error::Error, sync::Arc};

use tokio::{
    signal,
    sync::broadcast::{channel, Receiver},
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    bid_engine::BidEngine,
    chain::{ChainClient, ChainEvent},
    config::Config,
    data::{ModelRepo, ModelRepoFac},
    http::HttpServer,
};

mod bid_engine;
mod chain;
mod config;
mod data;
mod http;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

/// Spawns a critical task. If the task fails, the given token is cancelled.
async fn critical_task<F>(name: &str, token: CancellationToken, task: F) -> Result<()>
where
    F: Future<Output = Result<()>> + Send + 'static,
{
    task.await.map_err(|e| {
        tracing::error!("🚫 Critical task \"{name}\" failed: {e}");
        token.cancel();
        e
    })
}

trait TaskTrackerEx {
    fn spawn_chain_listener(
        &self,
        token: CancellationToken,
        chain_client: ChainClient,
    ) -> Receiver<ChainEvent>;

    fn spawn_http_server(
        &self,
        token: CancellationToken,
        port: u16,
        model_repo: Arc<dyn ModelRepo>,
    );

    fn spawn_bid_engine(
        &self,
        token: CancellationToken,
        chain_rx: Receiver<ChainEvent>,
        model_repo: Arc<dyn ModelRepo>,
    );

    fn spawn_shutdown_listener(&self, token: CancellationToken);
}

impl TaskTrackerEx for TaskTracker {
    fn spawn_chain_listener(
        &self,
        token: CancellationToken,
        chain_client: ChainClient,
    ) -> Receiver<ChainEvent> {
        let (chain_tx, chain_rx) = channel(128);

        self.spawn(critical_task("chain_listener", token.clone(), async move {
            chain_client.listen(token, chain_tx).await
        }));

        chain_rx
    }

    fn spawn_http_server(
        &self,
        token: CancellationToken,
        port: u16,
        model_repo: Arc<dyn ModelRepo>,
    ) {
        let http = HttpServer::new(port, model_repo);
        self.spawn(critical_task(
            "http_server",
            token.clone(),
            async move { http.serve(token).await },
        ));
    }

    fn spawn_bid_engine(
        &self,
        token: CancellationToken,
        chain_rx: Receiver<ChainEvent>,
        model_repo: Arc<dyn ModelRepo>,
    ) {
        let mut bid_engine = BidEngine::new(chain_rx, model_repo);
        self.spawn(critical_task("bid_engine", token.clone(), async move {
            bid_engine.run(token).await
        }));
    }

    fn spawn_shutdown_listener(&self, token: CancellationToken) {
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
                    tracing::info!("🚫 Received shutdown signal");
                    token.cancel();
                },
                _ = terminate => {
                    tracing::info!("🚫 Received termination signal");
                    token.cancel();
                },
                _ = token.cancelled() => {},
            }
        }

        self.spawn(shutdown_signal(token));
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::new();
    tracing_subscriber::registry().with(tracing_subscriber::fmt::layer()).init();

    let tracker = TaskTracker::new();
    let token = CancellationToken::new();
    tracker.spawn_shutdown_listener(token.clone());

    let chain_client = ChainClient::new(&config.airo_node).await.map_err(|e| {
        tracing::error!("🚫 Failed to connect to airo node: {e}");
        e
    })?;
    let chain_rx = tracker.spawn_chain_listener(token.clone(), chain_client);

    let model_repo = Arc::new(ModelRepoFac::in_memory());
    tracker.spawn_http_server(token.clone(), config.http_port, model_repo.clone());
    tracker.spawn_bid_engine(token, chain_rx, model_repo);

    tracker.close();
    tracker.wait().await;
    Ok(())
}
