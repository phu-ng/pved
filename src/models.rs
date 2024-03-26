use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Node {
    #[allow(dead_code)]
    status: String,
    id: String,
    pub(crate) node: String,
    r#type: String,
}

impl Node {
    pub fn new(status: String, id: String, node: String, r#type: String) -> Self {
        Self { status, id, node, r#type }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NodeData {
    pub(crate) data: Vec<Node>,
}

impl NodeData {
    pub fn new(data: Vec<Node>) -> Self {
        Self { data }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Qemu {
    status: String,
    name: String,
    pub(crate) tags: Option<String>,
    pub(crate) vmid: u32,
}

impl Qemu {
    pub fn new(status: String, name: String, tags: Option<String>, vmid: u32) -> Self {
        Self { status, name, tags, vmid }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct QemuData {
    pub(crate) data: Vec<Qemu>,
}

impl QemuData {
    pub fn new(data: Vec<Qemu>) -> Self {
        Self { data }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NetworkInterfaceData {
    pub(crate) data: NetworkInterfaceResult,
}

impl NetworkInterfaceData {
    pub fn new(data: NetworkInterfaceResult) -> Self {
        Self { data }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NetworkInterfaceResult {
    pub(crate) result: Vec<NetworkInterface>,
}

impl NetworkInterfaceResult {
    pub fn new(result: Vec<NetworkInterface>) -> Self {
        Self { result }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NetworkInterface {
    #[serde(rename = "hardware-address")]
    hardware_address: String,
    pub(crate) name: String,
    #[serde(rename = "ip-addresses")]
    pub(crate) ip_addresses: Vec<IpAddress>,
}

impl NetworkInterface {
    pub fn new(hardware_address: String, name: String, ip_addresses: Vec<IpAddress>) -> Self {
        Self { hardware_address, name, ip_addresses }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IpAddress {
    #[serde(rename = "ip-address-type")]
    pub(crate) ip_address_type: String,
    #[serde(rename = "ip-address")]
    pub(crate) ip_address: String,
    prefix: u32,
}

impl IpAddress {
    pub fn new(ip_address_type: String, ip_address: String, prefix: u32) -> Self {
        Self { ip_address_type, ip_address, prefix }
    }
}

#[derive(Debug, Serialize)]
pub struct Target {
    pub(crate) targets: Vec<String>,
    pub(crate) labels: Labels,
}

#[derive(Debug, Serialize)]
pub struct Labels {
    #[serde(rename = "__meta_prometheus_job")]
    pub(crate) meta_prometheus_job: String,
}