use std::{collections::HashMap, sync::OnceLock, thread::sleep, time::Duration};

use serde_json::Value;

use airo_wingman::cog::{Connector, Health};

use crate::common::{build_and_run, cmd, encode_file};

mod common;

type Port = u16;

#[derive(Copy, Clone)]
struct ModelPorts {
    hello_world: Port,
    resnet: Port,
}

static SETUP: OnceLock<ModelPorts> = OnceLock::new();
fn setup_models() -> ModelPorts {
    *SETUP.get_or_init(|| {
        println!("Setting up tests");
        cmd(
            "curl",
            ["-O", "https://storage.googleapis.com/tensorflow/keras-applications/resnet/resnet50_weights_tf_dim_ordering_tf_kernels.h5"],
            Some(".maintain/cog/resnet"),
        );
        let ports = ModelPorts {
            hello_world: build_and_run("hello-world", None),
            resnet: build_and_run("resnet", None),
        };
        sleep(Duration::from_secs(10));
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
    let input = HashMap::from([("image".to_owned(), encode_file(".maintain/cog/resnet/cat.png"))]);
    let prediction = connector.predict::<_, Value>(input).await.unwrap();
    assert!(prediction.output.is_some());
}
