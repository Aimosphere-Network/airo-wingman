//! TODO. Write docs.

#![warn(missing_docs)]

use std::{error::Error, future::Future, sync::Arc};

use tokio::{
    signal,
    sync::broadcast::{channel, Receiver},
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    bid_engine::BidEngine,
    chain::{ChainClient, ChainEvent, ChainListener, TxSubmitter},
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

#[tokio::main]
async fn main() -> Result<()> {
    let Config { http_port, airo_node, airo_suri } = Config::new();
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let tracker = TaskTracker::new();
    let token = CancellationToken::new();
    tracker.spawn_shutdown_listener(token.clone());

    let chain_client = ChainClient::new(&airo_node, &airo_suri).await.map_err(|e| {
        tracing::error!("🚫 Failed to connect to airo node: {e}");
        e
    })?;
    let chain_client = Arc::new(chain_client);
    let chain_rx = tracker.spawn_chain_listener(token.clone(), chain_client.clone());

    let model_repo = Arc::new(ModelRepoFac::in_memory());
    tracker.spawn_http_server(token.clone(), http_port, model_repo.clone());
    tracker.spawn_bid_engine(token, chain_rx, model_repo, chain_client);

    tracker.close();
    tracker.wait().await;
    Ok(())
}

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
        chain_listener: Arc<dyn ChainListener>,
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
        tx_sender: Arc<dyn TxSubmitter>,
    );

    fn spawn_shutdown_listener(&self, token: CancellationToken);
}

impl TaskTrackerEx for TaskTracker {
    fn spawn_chain_listener(
        &self,
        token: CancellationToken,
        chain_listener: Arc<dyn ChainListener>,
    ) -> Receiver<ChainEvent> {
        let (chain_tx, chain_rx) = channel(128);

        self.spawn(critical_task("chain_listener", token.clone(), async move {
            chain_listener.listen(token, chain_tx).await
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
        tx_sender: Arc<dyn TxSubmitter>,
    ) {
        let mut bid_engine = BidEngine::new(chain_rx, model_repo, tx_sender);
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
