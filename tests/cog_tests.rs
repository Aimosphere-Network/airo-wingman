use crate::common::{build_and_run, encode_file};
use airo_wingman::cog::{Connector, Health};
use serde_json::{json, Value};
use std::{collections::HashMap, sync::OnceLock, thread::sleep, time::Duration};

mod common;

type Port = u16;

#[derive(Copy, Clone)]
struct ModelPorts {
    hello_world: Port,
    resnet: Port,
    health_client: Port,
    health_server: Port,
}

static SETUP: OnceLock<ModelPorts> = OnceLock::new();
fn setup_models() -> ModelPorts {
    *SETUP.get_or_init(|| {
        println!("Setting up tests");
        let ports = ModelPorts {
            hello_world: build_and_run("hello-world", None),
            resnet: build_and_run("resnet", None),
            health_client: build_and_run("health-client", None),
            health_server: build_and_run("health-server", None),
        };
        sleep(Duration::from_secs(30));
        ports
    })
}

#[tokio::test]
async fn test_openapi_schema() {
    let model_ports = setup_models();
    let connector =
        Connector::new(&format!("http://localhost:{}", model_ports.hello_world)).unwrap();
    let schema = connector.openapi_schema().await.unwrap();
    assert_eq!(schema.openapi, "3.0.2");
}

#[tokio::test]
async fn test_health_check() {
    let model_ports = setup_models();
    let connector = Connector::new(&format!("http://localhost:{}", model_ports.resnet)).unwrap();
    let health = connector.health_check().await.unwrap();
    assert_eq!(health.status, Health::Ready);
}

#[tokio::test]
async fn test_ensure_ready() {
    let model_ports = setup_models();
    let connector = Connector::new(&format!("http://localhost:{}", model_ports.resnet)).unwrap();
    connector.ensure_ready().await.unwrap();
    let health = connector.health_check().await.unwrap();
    assert_eq!(health.status, Health::Ready);
}

#[tokio::test]
async fn test_predict_hello_world() {
    let model_ports = setup_models();
    let connector =
        Connector::new(&format!("http://localhost:{}", model_ports.hello_world)).unwrap();
    let input = HashMap::from([("text".to_owned(), "Dummy".to_owned())]);
    let prediction = connector.predict::<_, Value>(input).await.unwrap();
    assert_eq!(prediction.output.unwrap(), "hello Dummy");
}

#[tokio::test]
async fn test_predict_resnet() {
    let model_ports = setup_models();
    let connector = Connector::new(&format!("http://localhost:{}", model_ports.resnet)).unwrap();
    let input = HashMap::from([("image".to_owned(), encode_file("tests/cat.png"))]);
    let prediction = connector.predict::<_, Value>(input).await.unwrap();
    assert!(prediction.output.is_some());
}

#[tokio::test]
async fn test_predict_health() {
    let model_ports = setup_models();
    let client_connector =
        Connector::new(&format!("http://localhost:{}", model_ports.health_client)).unwrap();
    let symptoms = "1.1.1.1.1.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0";
    let input = json!({
        "encrypt": true,
        "input": symptoms
    });
    let client_encrypt = client_connector.predict::<_, Value>(input).await.unwrap();
    assert!(client_encrypt.output.is_some());
    // fs::write("tests/client_encrypt.json", client_encrypt.output.unwrap()).unwrap();

    let server_connector =
        Connector::new(&format!("http://localhost:{}", model_ports.health_server)).unwrap();
    let server_prediction =
        server_connector.predict::<_, Value>(client_encrypt.output).await.unwrap();
    assert!(server_prediction.output.is_some());
    // std::fs::write(
    //     "tests/server_prediction.json",
    //     serde_json::to_string(&server_prediction.output.unwrap()).unwrap(),
    // )
    // .unwrap();

    let client_decrypt = client_connector
        .predict::<_, Value>(json!({"encrypt": false, "input": server_prediction.output.unwrap()}))
        .await
        .unwrap();
    assert!(client_decrypt.output.is_some());
}
