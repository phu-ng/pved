mod models;
mod http_client_utils;

use std::collections::HashMap;
use actix_web::{get, App, HttpServer, HttpResponse};
use actix_web::http::header::{ContentType};
use reqwest::{Error, header};
use crate::models::{NodeData, QemuData, Target, Labels, NetworkInterfaceData, NetworkInterface};
use std::env;
use reqwest::Client;
use lazy_static::lazy_static;


lazy_static! {
    static ref HTTP_CLIENT: Client = http_client_utils::create_custom_http_client();
}

#[get("/")]
async fn discover() -> HttpResponse {
    // TODO: handle error khi khong tim thay bien - bo unwrap()
    let base_url = env::var("BASE_URL").unwrap();
    let api_key = env::var("API_KEY").unwrap();

    let nodes = get_nodes(base_url.as_str(), api_key.as_str()).await.unwrap();
    let qemus = get_qemus(base_url.as_str(), api_key.as_str(), nodes).await.unwrap();
    let ips = get_ips(base_url.as_str(), api_key.as_str(), qemus).await.unwrap();
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables from .env file.
    // Fails if .env file not found, not readable or invalid.
    dotenvy::dotenv().unwrap();

    HttpServer::new(|| {
        App::new()
            .service(discover)
    })
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}

async fn get_nodes(base_url: &str, api_key: &str) -> Result<Vec<String>, Error> {
    let client = &*HTTP_CLIENT;
    let resp = client.get(base_url.to_string() + "/nodes")
        .header(header::AUTHORIZATION, api_key)
        .send()
        .await?
        .json::<NodeData>()
        .await?;

    let mut nodes = vec![];
    for node in resp.data.into_iter() {
        nodes.push(node.node)
    }

    return Ok(nodes);
}

async fn get_qemus(base_url: &str, api_key: &str, nodes: Vec<String>) -> Result<HashMap<String, Vec<u32>>, Error> {
    let client = &*HTTP_CLIENT;
    let mut qemus = HashMap::new();

    for node in nodes {
        let mut qemu_ids = vec![];

        let resp = client.get(base_url.to_string() + "/nodes/" + node.as_str() + "/qemu")
            .header(header::AUTHORIZATION, api_key)
            .send()
            .await?
            .json::<QemuData>()
            .await?;

        for qemu in resp.data.into_iter() {
            if qemu.tags.is_none() {
                continue;
            }
            if qemu.tags.unwrap().contains("watch") {
                qemu_ids.push(qemu.vmid);
            }
        }

        qemus.entry(node.clone()).or_insert(qemu_ids);
    }

    return Ok(qemus);
}

async fn get_ips(base_url: &str, api_key: &str, vms: HashMap<String, Vec<u32>>) -> Result<Vec<String>, Error> {
    let client = &*HTTP_CLIENT;
    let mut ipv6s: Vec<String> = vec![];

    for (node, vm_ids) in vms {
        for id in vm_ids.iter() {
            let resp = client.get(base_url.to_string() + "/nodes/" + node.as_str() + "/qemu/" + id.to_string().as_str() + "/agent/network-get-interfaces")
                .header(header::AUTHORIZATION, api_key)
                .send()
                .await?
                .json::<NetworkInterfaceData>()
                .await?;

            let interfaces = resp.data.result;
            ipv6s = interfaces.iter()
                .filter(|interface|
                    interface.name.contains("eth") || interface.name.contains("ens"))
                .flat_map(|interface| get_ips_v6(interface).into_iter())
                .collect();
        }
    }

    return Ok(ipv6s);
}

fn get_ips_v6(interface: &NetworkInterface) -> Vec<String> {
    let mut ipv6s: Vec<String> = Vec::new();
    for ip in &interface.ip_addresses {
        if ip.ip_address_type == "ipv6" && is_public_ipv6(&ip.ip_address) {
            ipv6s.push(ip.ip_address.to_string());
        }
    }
    ipv6s
}

fn is_public_ipv6(ipv6: &String) -> bool {
    let private_prefixes = vec![
        "fd",
        "fc",
        "fe80",
    ];

    for prefix in private_prefixes {
        if ipv6.starts_with(prefix) {
            return false;
        }
    }

    return true;
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use super::*;

    #[tokio::test]
    async fn test_get_nodes() {
        let base_url = env::var("BASE_URL").unwrap().as_str();
        let api_key = env::var("API_KEY").unwrap().as_str();

        let nodes = get_nodes(base_url, api_key).await.unwrap();
        assert_eq!(nodes, vec!["x300", "k4"])
    }

    #[tokio::test]
    async fn test_get_qemus() {
        let base_url = env::var("BASE_URL").unwrap().as_str();
        let api_key = env::var("API_KEY").unwrap().as_str();
        let nodes = vec!["x300".to_string()];

        let qemus = get_qemus(base_url, api_key, nodes).await.unwrap();

        let mut result = HashMap::new();
        result.entry("x300".to_string()).or_insert(vec![201, 204]);
        assert_eq!(qemus, result)
    }

    #[tokio::test]
    async fn test_get_ips() {
        let base_url = env::var("BASE_URL").unwrap().as_str();
        let api_key = env::var("API_KEY").unwrap().as_str();

        let vm_ids = vec![201, 204];
        let mut vms = HashMap::new();
        vms.entry("x300".to_string()).or_insert(vm_ids);

        let ips = get_ips(base_url, api_key, vms).await.unwrap();

        assert_eq!(ips, vec!["2405:4803:fe85:1e0:be24:11ff:fe51:9939".to_string()])
    }
}
