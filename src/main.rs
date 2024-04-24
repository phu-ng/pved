mod models;
mod proxmox;
mod utils;

use actix_web::{get, App, HttpServer, HttpResponse};
use actix_web::http::header::{ContentType};
use crate::models::{Target, Labels};
use std::env;
use actix_web::middleware::Logger;
use reqwest::{Client};
use log::{error, info};
use serde_json::json;
use proxmox::{get_nodes, get_qemus, get_qemu_ips};
use crate::proxmox::{get_lxc_ips, get_lxcs};

#[derive(Clone)]
struct AppState {
    proxmox_http_client: Client,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables from .env file.
    // Fails if .env file not found, not readable or invalid.
    match dotenvy::dotenv() {
        Ok(_) => info!(".env file is found and loaded"),
        Err(_) => info!(".env file is not found")
    }

    // Enable logging
    // Set log level by adding env RUST_LOG=info/debug/...
    env_logger::init();

    let api_key = env::var("API_KEY").unwrap();
    let port = env::var("PORT").unwrap_or("8080".to_string());

    let data = AppState {
        proxmox_http_client: proxmox::default_proxmox_http_client(&api_key)
    };

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(actix_web::web::Data::new(data.clone()))
            .service(healthz)
            .service(discover)
    })
        .workers(2)
        .bind(format!("[::]:{}", port))?
        .run()
        .await
}

#[get("/healthz")]
async fn healthz() -> HttpResponse {
    let health = json!({"status": "OK"});

    HttpResponse::Ok()
        .content_type(ContentType::json())
        .insert_header(("Powered-By", "Rusty"))
        .json(health)
}

#[get("/")]
async fn discover(data: actix_web::web::Data<AppState>) -> HttpResponse {
    // TODO: handle error khi khong tim thay bien - bo unwrap()
    let base_url = env::var("BASE_URL").unwrap();

    let error_response = HttpResponse::InternalServerError()
        .insert_header(("PoweredBy", "Rusty"))
        .body("Internal Server Error");

    let nodes = match get_nodes(base_url.as_str(), &data.proxmox_http_client).await {
        Ok(data) => data,
        Err(e) => {
            error!("cannot get nodes from proxmox: {}", e.to_string());
            return error_response;
        }
    };
    let qemus = match get_qemus(base_url.as_str(), &data.proxmox_http_client, nodes.clone()).await {
        Ok(data) => data,
        Err(e) => {
            error!("cannot get qemus from proxmox: {}", e.to_string());
            return error_response;
        }
    };
    let ips = match get_qemu_ips(base_url.as_str(), &data.proxmox_http_client, qemus).await {
        Ok(data) => data,
        Err(e) => {
            error!("cannot get qemus network interfaces from proxmox: {}", e.to_string());
            return error_response;
        }
    };
    let lxcs = match get_lxcs(base_url.as_str(), &data.proxmox_http_client, nodes.clone()).await {
        Ok(data) => data,
        Err(e) => {
            error!("cannot get lxcs from proxmox: {}", e.to_string());
            return error_response;
        }
    };
    let lxc_ips = match get_lxc_ips(base_url.as_str(), &data.proxmox_http_client, lxcs).await {
        Ok(data) => data,
        Err(e) => {
            error!("cannot get lxc interfaces from proxmox: {}", e.to_string());
            return error_response;
        }
    };
    let mut targets = Vec::new();

    for ip in ips {
        targets.push(format!("[{}]:9100", ip));
    }
    for ip in lxc_ips {
        targets.push(format!("{}:9100", ip))
    }

    let target = Target {
        targets,
        labels: Labels {
            meta_prometheus_job: "proxmox".to_string()
        },
    };

    HttpResponse::Ok()
        .content_type(ContentType::json())
        .insert_header(("Powered-By", "Rusty"))
        .json(vec![target])
}

// TODO: Add tests