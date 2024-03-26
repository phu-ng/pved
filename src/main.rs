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
    use crate::models::{IpAddress, NetworkInterface, NetworkInterfaceResult, Node, Qemu};
    use super::*;

    #[tokio::test]
    async fn test_get_nodes_ok() {
        // Request a new server from the pool
        let mut server = mockito::Server::new_async().await;

        // Use one of these addresses to configure your client
        let url = server.url();
        let api_key = "1234";

        // Create a mock
        let node1 = Node::new("online".to_string(), "node/k4".to_string(), "k4".to_string(), "node".to_string());
        let node2 = Node::new("online".to_string(), "node/x300".to_string(), "x300".to_string(), "node".to_string());
        let nodes = vec![node1, node2];
        let node_data = NodeData::new(nodes);

        server.mock("GET", "/nodes")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_header("authorization", "1234")
            .with_body(serde_json::to_string(&node_data).unwrap())
            .create();

        let nodes = get_nodes(url.as_str(), api_key).await.unwrap();
        assert_eq!(nodes, vec!["k4", "x300"])
    }

    #[tokio::test]
    async fn test_get_qemus() {
        // Request a new server from the pool
        let mut server = mockito::Server::new_async().await;

        // Use one of these addresses to configure your client
        let url = server.url();
        let api_key = "1234";

        let nodes = vec!["node1".to_string(), "node2".to_string()];
        let node1_qemu_100 = Qemu::new("running".to_string(), "vm1".to_string(), Some("container;k8s".to_string()), 100);
        let node1_qemu_101 = Qemu::new("running".to_string(), "vm2".to_string(), Some("watch;database".to_string()), 101);
        let node2_qemu_200 = Qemu::new("running".to_string(), "vm3".to_string(), Some("k8s;watch".to_string()), 200);
        let node2_qemu_201 = Qemu::new("running".to_string(), "vm4".to_string(), Some("watch;iot".to_string()), 201);
        let node1_qemus = QemuData::new(vec![node1_qemu_100, node1_qemu_101]);
        let node2_qemus = QemuData::new(vec![node2_qemu_200, node2_qemu_201]);

        server.mock("GET", "/nodes/node1/qemu")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_header("authorization", "1234")
            .with_body(serde_json::to_string(&node1_qemus).unwrap())
            .create();

        server.mock("GET", "/nodes/node2/qemu")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_header("authorization", "1234")
            .with_body(serde_json::to_string(&node2_qemus).unwrap())
            .create();

        let mut expected = HashMap::new();
        expected.entry("node1".to_string()).or_insert(vec![101]);
        expected.entry("node2".to_string()).or_insert(vec![200, 201]);

        let qemu_ids = get_qemus(url.as_str(), api_key, nodes).await.unwrap();
        assert_eq!(qemu_ids, expected)
    }

    #[tokio::test]
    async fn test_get_ips() {
        // Request a new server from the pool
        let mut server = mockito::Server::new_async().await;

        // Use one of these addresses to configure your client
        let url = server.url();
        let api_key = "1234";

        let mut qemu_ids = HashMap::new();
        qemu_ids.entry("node1".to_string()).or_insert(vec![101, 102]);

        let qemu_101_net_interface_eth0 = NetworkInterface::new("00:00:00:00:00:00".to_string(), "eth0".to_string(),
                                                                vec![IpAddress::new("ipv4".to_string(), "192.168.1.34".to_string(), 24),
                                                                     IpAddress::new("ipv6".to_string(), "fdbc:81e7:7a9a::".to_string(), 48)]);
        let qemu_101_net_interface_lo = NetworkInterface::new("00:00:00:00:00:00".to_string(), "lo".to_string(),
                                                              vec![IpAddress::new("ipv4".to_string(), "127.0.0.1".to_string(), 8),
                                                                   IpAddress::new("ipv6".to_string(), "::1".to_string(), 128)]);
        let qemu_101_net_interface_docker = NetworkInterface::new("00:00:00:00:00:00".to_string(), "docker0".to_string(),
                                                                  vec![IpAddress::new("ipv4".to_string(), "172.16.0.1".to_string(), 16)]);

        let qemu_102_net_interface_ens18 = NetworkInterface::new("00:00:00:00:00:00".to_string(), "ens18".to_string(),
                                                                 vec![IpAddress::new("ipv4".to_string(), "192.168.1.35".to_string(), 24),
                                                                      IpAddress::new("ipv6".to_string(), "2404:6800:4005:815::200e".to_string(), 64),
                                                                      IpAddress::new("ipv6".to_string(), "2504:6800:4005:815::200d".to_string(), 64)]);
        let qemu_102_net_interface_lo = qemu_101_net_interface_lo.clone();
        let qemu_102_net_interface_docker = qemu_101_net_interface_docker.clone();

        let qemu_101_net_interface_data = NetworkInterfaceData::new(
            NetworkInterfaceResult::new(vec![qemu_101_net_interface_docker, qemu_101_net_interface_eth0, qemu_101_net_interface_lo]));
        let qemu_102_net_interface_data = NetworkInterfaceData::new(
            NetworkInterfaceResult::new(vec![qemu_102_net_interface_docker, qemu_102_net_interface_lo, qemu_102_net_interface_ens18]));

        server.mock("GET", "/nodes/node1/qemu/101/agent/network-get-interfaces")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_header("authorization", "1234")
            .with_body(serde_json::to_string(&qemu_101_net_interface_data).unwrap())
            .create();

        server.mock("GET", "/nodes/node1/qemu/102/agent/network-get-interfaces")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_header("authorization", "1234")
            .with_body(serde_json::to_string(&qemu_102_net_interface_data).unwrap())
            .create();

        let ips = get_ips(url.as_str(), api_key, qemu_ids).await.unwrap();
        assert_eq!(ips, vec!["2404:6800:4005:815::200e".to_string()])
    }
}
