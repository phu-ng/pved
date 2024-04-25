mod models;
mod proxmox;
mod utils;

use actix_web::{get, App, HttpServer, HttpResponse};
use actix_web::http::header::{ContentType};
use crate::models::{Target, Labels, VmType};
use std::env;
use actix_web::middleware::Logger;
use reqwest::Client;
use log::{error, info};
use serde_json::json;
use proxmox::get_nodes;
use crate::proxmox::get_vm;
use crate::utils::generate_fqdn;

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
    let domain = env::var("DOMAIN").unwrap_or("phu.homes".to_string());

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

    let mut vms = vec![];
    for node in nodes {
        match get_vm(base_url.as_str(), &data.proxmox_http_client, &node, VmType::LXC).await {
            Ok(data) => vms.extend(data),
            Err(e) => {
                error!("cannot get vms from proxmox: {}", e.to_string());
                return error_response;
            }
        };

        match get_vm(base_url.as_str(), &data.proxmox_http_client, &node, VmType::QEMU).await {
            Ok(data) => vms.extend(data),
            Err(e) => {
                error!("cannot get vms from proxmox: {}", e.to_string());
                return error_response;
            }
        };
    }

    let mut targets = vec![];
    for vm in vms {
        let fqdn = generate_fqdn(&vm.name, &domain);

        let fqdns = vec![format!("{}:9100", fqdn)];
        let target = Target {
            targets: fqdns,
            labels: Labels {
                meta_prometheus_job: "proxmox".to_string(),
                vm_type: vm.vm_type.unwrap_or("qemu".to_string()),
                hostname: vm.name,
            },
        };

        targets.push(target);
    }

    HttpResponse::Ok()
        .content_type(ContentType::json())
        .insert_header(("Powered-By", "Rusty"))
        .json(targets)
}

// TODO: Add tests