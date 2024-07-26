use airo_wingman::{
    chain::{ChainClient, DataExchange},
    types::ContentId,
};

// TODO. Start airo-node automatically.
#[ignore]
#[tokio::test]
async fn test_data_exchange() {
    let data = vec![1, 2, 3];
    let content_id = ContentId::random();
    let client = ChainClient::new("ws://localhost:9945", "//Alice").await.unwrap();
    assert!(client.upload(content_id, data.clone()).await.is_ok());
    assert_eq!(Some(data), client.download(content_id).await.unwrap());
}
