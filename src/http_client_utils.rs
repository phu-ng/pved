use std::time::Duration;
use reqwest::{Client, ClientBuilder};

pub fn create_custom_http_client() -> Client {
    let builder = ClientBuilder::new();
    let client = builder
        .timeout(Duration::from_secs(60))
        .tcp_keepalive(Duration::from_secs(60))// Set the timeout for both connect and read operations
        .build()
        .expect("Failed to create custom HTTP client");
    client
}