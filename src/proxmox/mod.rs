use std::collections::HashMap;
use std::time::Duration;
use reqwest::{Client, ClientBuilder, Error, header};
use crate::models::{IpAddress, NetworkInterface, NetworkInterfaceData, NodeData, QemuData, LxcInterface, LxcInterfaceData, LxcData};
use crate::utils::is_public_ipv6;

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

pub async fn get_nodes(base_url: &str, proxmox_http_client: &Client) -> reqwest::Result<Vec<String>> {
    let resp = proxmox_http_client.get(base_url.to_string() + "/nodes")
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

pub async fn get_qemus(base_url: &str, proxmox_http_client: &Client, nodes: Vec<String>) -> Result<HashMap<String, Vec<u32>>, Error> {
    let mut qemus = HashMap::new();

    for node in nodes {
        let mut qemu_ids = vec![];

        let resp = proxmox_http_client.get(base_url.to_string() + "/nodes/" + node.as_str() + "/qemu")
            .send()
            .await?
            .json::<QemuData>()
            .await?;

        for qemu in resp.data.into_iter() {
            if qemu.tags.is_none() {
                continue;
            }
            if qemu.tags.unwrap().contains("watch") && qemu.status.contains("running") {
                qemu_ids.push(qemu.vmid);
            }
        }

        qemus.entry(node.clone()).or_insert(qemu_ids);
    }

    return Ok(qemus);
}

pub async fn get_qemu_ips(base_url: &str, proxmox_http_client: &Client, qemus: HashMap<String, Vec<u32>>) -> Result<Vec<String>, Error> {
    let mut interfaces: Vec<NetworkInterface> = vec![];

    for (node, qemu_ids) in qemus {
        for id in qemu_ids.iter() {
            let resp = proxmox_http_client.get(base_url.to_string() + "/nodes/" + node.as_str() + "/qemu/" + id.to_string().as_str() + "/agent/network-get-interfaces")
                .send()
                .await?
                .json::<NetworkInterfaceData>()
                .await?;

            interfaces.extend(resp.data.result);
        }
    }

    interfaces.retain(|interface| interface.name.contains("eth") || interface.name.contains("ens"));
    let mut addresses: Vec<IpAddress> = vec![];
    for interface in interfaces {
        addresses.extend(interface.ip_addresses);
    }

    addresses.retain(|address| address.ip_address_type == "ipv6" && is_public_ipv6(&address.ip_address));
    let ipv6s = addresses.iter()
        .map(|address| &address.ip_address).cloned()
        .collect();

    return Ok(ipv6s);
}

pub async fn get_lxcs(base_url: &str, proxmox_http_client: &Client, nodes: Vec<String>) -> Result<HashMap<String, Vec<String>>, Error> {
    let mut lxcs = HashMap::new();

    for node in nodes {
        let mut ids = vec![];

        let resp = proxmox_http_client.get(base_url.to_string() + "/nodes/" + node.as_str() + "/lxc")
            .send()
            .await?
            .json::<LxcData>()
            .await?;

        for lxc in resp.data.into_iter() {
            if lxc.tags.is_none() {
                continue;
            }
            if lxc.tags.unwrap().contains("watch") && lxc.status.contains("running") {
                ids.push(lxc.vmid);
            }
        }

        lxcs.entry(node.clone()).or_insert(ids);
    }

    return Ok(lxcs);
}

pub async fn get_lxc_ips(base_url: &str, proxmox_http_client: &Client, lxcs: HashMap<String, Vec<String>>) -> Result<Vec<String>, Error> {
    let mut interfaces: Vec<LxcInterface> = vec![];

    for (node, ids) in lxcs {
        for id in ids.iter() {
            let resp = proxmox_http_client.get(base_url.to_string() + "/nodes/" + node.as_str() + "/lxc/" + id.as_str() + "/interfaces")
                .send()
                .await?
                .json::<LxcInterfaceData>()
                .await?;

            interfaces.extend(resp.data);
        }
    }

    interfaces.retain(|interface| interface.name.contains("eth") || interface.name.contains("ens"));
    let mut ipv4: Vec<String> = vec![];
    for interface in interfaces {
        ipv4.push(interface.inet.split("/").next().unwrap().to_string());
    }

    return Ok(ipv4);
}

#[cfg(test)]
mod tests {
    use crate::models::{IpAddress, Lxc, NetworkInterface, NetworkInterfaceResult, Node, Qemu};
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
            .with_body(serde_json::to_string(&node_data).unwrap())
            .create();

        let proxmox_http_client = default_proxmox_http_client(api_key);
        let nodes = get_nodes(url.as_str(), &proxmox_http_client).await.unwrap();
        assert_eq!(nodes, vec!["k4", "x300"])
    }

    #[tokio::test]
    async fn test_get_qemus() {
        // Request a new server from the pool
        let mut server = mockito::Server::new_async().await;

        // Use one of these addresses to configure your client
        let url = server.url();
        let api_key = "1234";
        let proxmox_http_client = default_proxmox_http_client(api_key);

        let nodes = vec!["node1".to_string(), "node2".to_string()];
        let node1_qemu_100 = Qemu::new("running".to_string(), "vm1".to_string(), Some("container;k8s".to_string()), 100);
        let node1_qemu_101 = Qemu::new("running".to_string(), "vm2".to_string(), Some("watch;database".to_string()), 101);
        let node2_qemu_200 = Qemu::new("running".to_string(), "vm3".to_string(), Some("k8s;watch".to_string()), 200);
        let node2_qemu_201 = Qemu::new("running".to_string(), "vm4".to_string(), Some("watch;iot".to_string()), 201);
        let node2_qemu_202 = Qemu::new("stopped".to_string(), "vm4".to_string(), Some("watch".to_string()), 202);
        let node1_qemus = QemuData::new(vec![node1_qemu_100, node1_qemu_101]);
        let node2_qemus = QemuData::new(vec![node2_qemu_200, node2_qemu_201, node2_qemu_202]);

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

        let qemu_ids = get_qemus(url.as_str(), &proxmox_http_client, nodes).await.unwrap();
        assert_eq!(qemu_ids, expected)
    }

    #[tokio::test]
    async fn test_get_qemu_ips() {
        // Request a new server from the pool
        let mut server = mockito::Server::new_async().await;

        // Use one of these addresses to configure your client
        let url = server.url();
        let api_key = "1234";
        let proxmox_http_client = default_proxmox_http_client(api_key);

        let mut qemu_ids = HashMap::new();
        qemu_ids.entry("node1".to_string()).or_insert(vec![101, 102, 103]);

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
                                                                      IpAddress::new("ipv6".to_string(), "2404:6800:4005:815::200e".to_string(), 64)]);
        let qemu_102_net_interface_lo = qemu_101_net_interface_lo.clone();
        let qemu_102_net_interface_docker = qemu_101_net_interface_docker.clone();

        let qemu_103_net_interface_ens18 = NetworkInterface::new("00:00:00:00:00:00".to_string(), "ens18".to_string(),
                                                                 vec![IpAddress::new("ipv4".to_string(), "192.168.1.36".to_string(), 24),
                                                                      IpAddress::new("ipv6".to_string(), "2404:6800:4005:815::200d".to_string(), 64)]);

        let qemu_101_net_interface_data = NetworkInterfaceData::new(
            NetworkInterfaceResult::new(vec![qemu_101_net_interface_docker, qemu_101_net_interface_eth0, qemu_101_net_interface_lo]));
        let qemu_102_net_interface_data = NetworkInterfaceData::new(
            NetworkInterfaceResult::new(vec![qemu_102_net_interface_docker, qemu_102_net_interface_lo, qemu_102_net_interface_ens18]));
        let qemu_103_net_interface_data = NetworkInterfaceData::new(
            NetworkInterfaceResult::new(vec![qemu_103_net_interface_ens18]));

        server.mock("GET", "/nodes/node1/qemu/101/agent/network-get-interfaces")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&qemu_101_net_interface_data).unwrap())
            .create();

        server.mock("GET", "/nodes/node1/qemu/102/agent/network-get-interfaces")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&qemu_102_net_interface_data).unwrap())
            .create();

        server.mock("GET", "/nodes/node1/qemu/103/agent/network-get-interfaces")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&qemu_103_net_interface_data).unwrap())
            .create();

        let ips = get_qemu_ips(url.as_str(), &proxmox_http_client, qemu_ids).await.unwrap();
        assert_eq!(ips, vec!["2404:6800:4005:815::200e".to_string(), "2404:6800:4005:815::200d".to_string()])
    }

    #[tokio::test]
    async fn test_get_lxcs() {
        // Request a new server from the pool
        let mut server = mockito::Server::new_async().await;

        // Use one of these addresses to configure your client
        let url = server.url();
        let api_key = "1234";
        let proxmox_http_client = default_proxmox_http_client(api_key);

        let nodes = vec!["node1".to_string(), "node2".to_string()];
        let node1_lxc_100 = Lxc { status: "running".to_string(), vmid: "100".to_string(), tags: Some("container;k8s".to_string()) };
        let node1_lxc_101 = Lxc { status: "running".to_string(), vmid: "101".to_string(), tags: Some("watch;database".to_string()) };
        let node2_lxc_200 = Lxc { status: "running".to_string(), vmid: "200".to_string(), tags: Some("k8s;watch".to_string()) };
        let node2_lxc_201 = Lxc { status: "running".to_string(), vmid: "201".to_string(), tags: Some("watch;iot".to_string()) };
        let node2_lxc_202 = Lxc { status: "stopped".to_string(), vmid: "202".to_string(), tags: Some("watch".to_string()) };
        let node1_lxcs = LxcData { data: vec![node1_lxc_100, node1_lxc_101] };
        let node2_lxcs = LxcData { data: vec![node2_lxc_200, node2_lxc_201, node2_lxc_202] };

        server.mock("GET", "/nodes/node1/lxc")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_header("authorization", "1234")
            .with_body(serde_json::to_string(&node1_lxcs).unwrap())
            .create();

        server.mock("GET", "/nodes/node2/lxc")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_header("authorization", "1234")
            .with_body(serde_json::to_string(&node2_lxcs).unwrap())
            .create();

        let mut expected = HashMap::new();
        expected.entry("node1".to_string()).or_insert(vec!["101".to_string()]);
        expected.entry("node2".to_string()).or_insert(vec!["200".to_string(), "201".to_string()]);

        let ids = get_lxcs(url.as_str(), &proxmox_http_client, nodes).await.unwrap();
        assert_eq!(ids, expected)
    }
}
