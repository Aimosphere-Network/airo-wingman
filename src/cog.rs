use std::collections::HashMap;

use openapiv3::OpenAPI;
use reqwest::{Client, Response, StatusCode, Url};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use tokio::time::sleep;

use crate::types::{stdResult, Result};

#[derive(Debug, Deserialize)]
pub struct ValidationError {
    #[serde(rename = "loc")]
    pub location: Vec<String>,
    #[serde(rename = "msg")]
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
}

#[derive(Debug, Deserialize)]
pub struct HTTPValidationError {
    pub detail: Vec<ValidationError>,
}

/// Status of setup or prediction.
#[derive(Debug, PartialEq, Deserialize, strum::Display)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Starting,
    Processing,
    Succeeded,
    Canceled,
    Failed,
}

/// Result of a model setup.
#[derive(Debug, Deserialize)]
pub struct SetupResult {
    /// Setup started time
    pub started_at: String,
    /// Setup completed time
    pub completed_at: String,
    /// Setup logs
    pub logs: String,
    /// Setup status
    pub status: Status,
}

/// Health status.
#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Health {
    Unknown,
    Starting,
    Ready,
    Busy,
    SetupFailed,
}

/// Health check.
#[derive(Debug, Deserialize)]
pub struct HealthCheck {
    /// Current health status
    pub status: Health,
    /// Setup information
    pub setup: SetupResult,
}

#[derive(Debug, Deserialize)]
pub struct PredictionResponse<In = Value, Out = Value> {
    pub input: Option<In>,
    pub output: Option<Out>,
    pub id: Option<String>,
    pub version: Option<String>,
    pub created_at: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub logs: String,
    pub error: Option<String>,
    pub status: Status,
    pub metrics: Option<HashMap<String, Value>>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Model setup failed")]
    SetupFailed,
    #[error("Input validation failed")]
    InputValidation { errors: Vec<ValidationError> },
}

/// Cog Connector. Connects to the Cog API and performs health checks and predictions.
pub struct Connector {
    url: Url,
    http: Client,
}

impl Connector {
    /// Create a new Cog Connector from a base url of a Cog API.
    pub fn new(url: &str) -> Result<Connector> {
        let url = Url::parse(url)?;
        let http = Client::new();
        Ok(Connector { url, http })
    }

    /// Get OpenAPI schema.
    pub async fn openapi_schema(&self) -> Result<OpenAPI> {
        let url = self.url.join("openapi.json")?;
        self.http
            .get(url)
            .send()
            .await
            .and_then(Response::error_for_status)?
            .json::<OpenAPI>()
            .await
            .map_err(Into::into)
    }

    /// Perform a health check.
    pub async fn health_check(&self) -> Result<HealthCheck> {
        let url = self.url.join("health-check")?;
        self.http
            .get(url)
            .send()
            .await
            .and_then(Response::error_for_status)?
            .json::<HealthCheck>()
            .await
            .map_err(Into::into)
    }

    /// Ensure that the Cog API is ready by performing a health check in a loop every 1 second.
    /// This function will block until the Cog API is ready. This function will return an error if
    /// the Cog API returns [Health::SetupFailed].
    pub async fn ensure_ready(&self) -> stdResult<(), Error> {
        loop {
            if let Ok(health) = self.health_check().await.map_err(|e| {
                tracing::warn!("Cog Health check failed: {}.\nRetrying...", e);
            }) {
                match health.status {
                    Health::Ready => return Ok(()),
                    Health::SetupFailed => return Err(Error::SetupFailed),
                    _ => {},
                }
            }

            sleep(std::time::Duration::from_secs(1)).await;
        }
    }

    /// Make a prediction. This function uses the `POST /predictions` endpoint of the Cog API to
    /// make a prediction. This method is blocking.
    pub async fn predict<In, Out>(&self, inputs: In) -> Result<PredictionResponse<In, Out>>
    where
        In: Serialize + DeserializeOwned,
        Out: DeserializeOwned,
    {
        let url = self.url.join("predictions")?;
        let req = json!({ "input": inputs });

        let res = self.http.post(url).json(&req).send().await?;
        if res.status() == StatusCode::UNPROCESSABLE_ENTITY {
            let errors = res.json::<HTTPValidationError>().await?.detail;
            return Err(Error::InputValidation { errors }.into());
        }

        res.error_for_status()?
            .json::<PredictionResponse<In, Out>>()
            .await
            .map_err(Into::into)
    }
}
