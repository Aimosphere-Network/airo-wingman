use std::{collections::HashMap, sync::OnceLock, thread::sleep, time::Duration};

use serde_json::Value;

use airo_wingman::cog::{Connector, Health};

use crate::common::build_and_run;

mod common;

static SETUP: OnceLock<u16> = OnceLock::new();
fn setup_tests() -> u16 {
    let port = SETUP.get_or_init(|| {
        println!("Setting up tests");
        let port = build_and_run("hello-world", None);
        sleep(Duration::from_secs(10));
        port
    });
    *port
}

#[tokio::test]
async fn test_openapi_schema() {
    let port = setup_tests();
    let connector = Connector::new(&format!("http://localhost:{port}")).unwrap();
    let schema = connector.openapi_schema().await.unwrap();
    assert_eq!(schema.openapi, "3.0.2");
}

#[tokio::test]
async fn test_health_check() {
    let port = setup_tests();
    let connector = Connector::new(&format!("http://localhost:{port}")).unwrap();
    let health = connector.health_check().await.unwrap();
    assert_eq!(health.status, Health::Ready);
}

#[tokio::test]
async fn test_ensure_ready() {
    let port = setup_tests();
    let connector = Connector::new(&format!("http://localhost:{port}")).unwrap();
    connector.ensure_ready().await.unwrap();
    let health = connector.health_check().await.unwrap();
    assert_eq!(health.status, Health::Ready);
}

#[tokio::test]
async fn test_predict() {
    let port = setup_tests();
    let connector = Connector::new(&format!("http://localhost:{port}")).unwrap();
    let input = HashMap::from([("text".to_owned(), "Dummy".to_owned())]);
    let prediction = connector.predict::<_, Value>(input).await.unwrap();
    assert_eq!(prediction.output.unwrap(), "hello Dummy");
}
