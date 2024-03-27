use std::time::Duration;
use reqwest::{Client, ClientBuilder, header};

pub fn default_proxmox_http_client(api_key: &str) -> Client {
    let mut headers = header::HeaderMap::new();
    headers.insert(header::AUTHORIZATION, api_key.parse().unwrap());

    let builder = ClientBuilder::new();
    let client = builder
        .timeout(Duration::from_secs(2))
        .tcp_keepalive(Duration::from_secs(60))
        .default_headers(headers)// Set the timeout for both connect and read operations
        .build()
        .expect("Failed to create custom HTTP client");
    client
}