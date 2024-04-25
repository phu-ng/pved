use std::time::Duration;
use reqwest::{Client, ClientBuilder, Error, header};
use crate::models::{NodeData, VmType, Vm, VmData};

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
        if node.status != "online" {
            continue;
        }
        nodes.push(node.node)
    }

    return Ok(nodes);
}

pub async fn get_vm(base_url: &str, proxmox_http_client: &Client, node: &str, vm_type: VmType) -> Result<Vec<Vm>, Error> {
    let mut vms = vec![];

    let resp = proxmox_http_client.get(base_url.to_string() + "/nodes/" + node + "/" + vm_type.as_str())
        .send()
        .await?
        .json::<VmData>()
        .await?;

    for vm in resp.data.into_iter() {
        let tags = vm.tags.clone();
        let status = vm.status.clone();

        if tags.is_none() {
            continue;
        }
        if tags.unwrap().contains("watch") && status.contains("running") {
            vms.push(vm.clone());
        }
    }

    Ok(vms)
}

#[cfg(test)]
mod tests {
    use crate::models::{IpAddress, Lxc, NetworkInterface, NetworkInterfaceResult, Node, Qemu, VmId};
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
        let node3 = Node::new("offline".to_string(), "node/x301".to_string(), "x301".to_string(), "node".to_string());
        let nodes = vec![node1, node2, node3];
        let node_data = NodeData::new(nodes);

        server.mock("GET", "/nodes")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&node_data).unwrap())
            .create();

        let proxmox_http_client = default_proxmox_http_client(api_key);
        let nodes = get_nodes(url.as_str(), &proxmox_http_client).await.unwrap();
        assert_eq!(nodes, vec!["k4", "x300"])
    }

    #[tokio::test]
    async fn test_get_vm() {
        // Request a new server from the pool
        let mut server = mockito::Server::new_async().await;

        // Use one of these addresses to configure your client
        let url = server.url();
        let api_key = "1234";
        let node = "node1";
        let proxmox_http_client = default_proxmox_http_client(api_key);

        let node_vm_100 = Vm { status: "running".to_string(), name: "vm1".to_string(), tags: Some("container;k8s".to_string()), vm_id: VmId::Int(100), vm_type: Some("lxc".to_string()) };
        let node_vm_101 = Vm { status: "stopped".to_string(), name: "vm2".to_string(), tags: Some("watch;database".to_string()), vm_id: VmId::Int(101), vm_type: Some("lxc".to_string()) };
        let node_vm_102 = Vm { status: "running".to_string(), name: "vm3".to_string(), tags: Some("watch;k8s".to_string()), vm_id: VmId::Text("102".to_string()), vm_type: None };
        let node_vm_103 = Vm { status: "stopped".to_string(), name: "vm4".to_string(), tags: Some("watch;database".to_string()), vm_id: VmId::Text("103".to_string()), vm_type: None };
        let node_vm_lxc = VmData { data: vec![node_vm_100.clone(), node_vm_101] };
        let node_vm_qemu = VmData { data: vec![node_vm_102.clone(), node_vm_103] };

        server.mock("GET", "/nodes/node1/lxc")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_header("authorization", "1234")
            .with_body(serde_json::to_string(&node_vm_lxc).unwrap())
            .create();

        server.mock("GET", "/nodes/node1/qemu")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_header("authorization", "1234")
            .with_body(serde_json::to_string(&node_vm_qemu).unwrap())
            .create();

        let mut expected = vec![];
        expected.push(node_vm_102);

        let vms = get_vm(url.as_str(), &proxmox_http_client, node, VmType::LXC).await.unwrap();
        let vms = get_vm(url.as_str(), &proxmox_http_client, node, VmType::QEMU).await.unwrap();
        assert_eq!(vms, expected)
    }
}
