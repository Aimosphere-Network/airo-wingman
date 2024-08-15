use airo_wingman::protocol::{AiroClient, DataExchange};

// TODO. Start airo-node automatically.
#[ignore]
#[tokio::test]
async fn test_data_exchange() {
    let data = vec![1, 2, 3];
    let client = AiroClient::new("ws://localhost:9944", "//Alice").await.unwrap();
    let content_id = client.hash_upload(data.clone()).await;
    assert!(content_id.is_ok());
    assert_eq!(Some(data), client.download(content_id.unwrap()).await.unwrap());
}
