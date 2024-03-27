mod models;
mod proxmox;
mod utils;

use actix_web::{get, App, HttpServer, HttpResponse};
use actix_web::http::header::{ContentType};
use crate::models::{Target, Labels};
use std::env;
use proxmox::{get_nodes, get_qemus, get_ips};

#[derive(Clone)]
struct AppState {
    proxmox_http_client: reqwest::Client,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables from .env file.
    // Fails if .env file not found, not readable or invalid.
    dotenvy::dotenv().unwrap();

    // Enable logging
    // Set log level by adding env RUST_LOG=info/debug/...
    env_logger::init();

    let api_key = env::var("API_KEY").unwrap();
    let data = AppState {
        proxmox_http_client: proxmox::default_proxmox_http_client(&api_key)
    };

    HttpServer::new(move || {
        App::new()
            .app_data(actix_web::web::Data::new(data.clone()))
            .service(discover)
    })
        .workers(2)
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}

#[get("/")]
async fn discover(data: actix_web::web::Data<AppState>) -> HttpResponse {
    // TODO: handle error khi khong tim thay bien - bo unwrap()
    let base_url = env::var("BASE_URL").unwrap();

    let nodes = get_nodes(base_url.as_str(), &data.proxmox_http_client).await.unwrap();
    let qemus = get_qemus(base_url.as_str(), &data.proxmox_http_client, nodes).await.unwrap();
    let ips = get_ips(base_url.as_str(), &data.proxmox_http_client, qemus).await.unwrap();
    let mut targets = Vec::new();

    for ip in ips {
        targets.push(format!("[{}]:9100", ip));
    }

    let target = Target {
        targets,
        labels: Labels {
            meta_prometheus_job: "proxmox".to_string()
        },
    };

    HttpResponse::Ok()
        .content_type(ContentType::json())
        .insert_header(("PoweredBy", "Rusty"))
        .json(vec![target])
}
