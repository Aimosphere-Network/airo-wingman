//! TODO. Write docs.

#![allow(dead_code)]

use std::{future::Future, sync::Arc};

use tokio::{
    signal,
    sync::broadcast::{channel, Receiver, Sender},
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    config::Config,
    data::{ModelRepo, ModelRepoFac},
    engine::{BidEngine, Engine, ExecutionEngine},
    http::HttpServer,
    protocol::{AiroClient, ChainEvent, ChainListener, Protocol, TxSubmitter},
    types::Result,
};

pub mod cog;
pub mod config;
pub mod data;
pub mod engine;
pub mod http;
pub mod protocol;
pub mod types;
pub mod utils;

pub async fn start() -> Result<()> {
    let Config { http_port, airo_node, airo_suri } = Config::new();
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let tracker = TaskTracker::new();
    let token = CancellationToken::new();
    tracker.spawn_shutdown_listener(token.clone());

    let airo_client = AiroClient::new(&airo_node, &airo_suri).await.map_err(|e| {
        tracing::error!("🚫 Failed to connect to airo node: {e}");
        e
    })?;
    let airo_client = Arc::new(airo_client);
    let (chain_tx, chain_rx_bid) = channel(128);
    let chain_rx_exec = chain_tx.subscribe();
    tracker.spawn_chain_listener(token.clone(), airo_client.clone(), chain_tx);

    let model_repo = Arc::new(ModelRepoFac::in_memory());
    tracker.spawn_http_server(token.clone(), http_port, model_repo.clone());
    tracker.spawn_execution_engine(
        token.clone(),
        chain_rx_exec,
        airo_client.clone(),
        model_repo.clone(),
    );
    tracker.spawn_bid_engine(token, chain_rx_bid, airo_client, model_repo);

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
        chain_listener: Arc<dyn ChainListener + Send + Sync>,
        chain_tx: Sender<ChainEvent>,
    );

    fn spawn_http_server(
        &self,
        token: CancellationToken,
        port: u16,
        model_repo: Arc<dyn ModelRepo + Send + Sync>,
    );

    fn spawn_bid_engine(
        &self,
        token: CancellationToken,
        chain_rx: Receiver<ChainEvent>,
        tx_sender: Arc<dyn TxSubmitter + Send + Sync>,
        model_repo: Arc<dyn ModelRepo + Send + Sync>,
    );

    fn spawn_execution_engine(
        &self,
        token: CancellationToken,
        chain_rx: Receiver<ChainEvent>,
        protocol_client: Arc<dyn Protocol + Send + Sync>,
        model_repo: Arc<dyn ModelRepo + Send + Sync>,
    );

    fn spawn_shutdown_listener(&self, token: CancellationToken);
}

impl TaskTrackerEx for TaskTracker {
    fn spawn_chain_listener(
        &self,
        token: CancellationToken,
        chain_listener: Arc<dyn ChainListener + Send + Sync>,
        chain_tx: Sender<ChainEvent>,
    ) {
        self.spawn(critical_task("chain_listener", token.clone(), async move {
            chain_listener.listen(token, chain_tx).await
        }));
    }

    fn spawn_http_server(
        &self,
        token: CancellationToken,
        port: u16,
        model_repo: Arc<dyn ModelRepo + Send + Sync>,
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
        tx_sender: Arc<dyn TxSubmitter + Send + Sync>,
        model_repo: Arc<dyn ModelRepo + Send + Sync>,
    ) {
        let mut bid_engine = BidEngine::new(chain_rx, tx_sender, model_repo);
        self.spawn(critical_task("bid_engine", token.clone(), async move {
            bid_engine.run(token).await
        }));
    }

    fn spawn_execution_engine(
        &self,
        token: CancellationToken,
        chain_rx: Receiver<ChainEvent>,
        protocol_client: Arc<dyn Protocol + Send + Sync>,
        model_repo: Arc<dyn ModelRepo + Send + Sync>,
    ) {
        let mut execution_engine = ExecutionEngine::new(chain_rx, protocol_client, model_repo);
        self.spawn(critical_task("execution_engine", token.clone(), async move {
            execution_engine.run(token).await
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
