use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct Node {
    #[allow(dead_code)]
    status: String,
    id: String,
    ssl_fingerprint: String,
    pub(crate) node: String,
    r#type: String,
    level: String,
}

#[derive(Debug, Deserialize)]
pub struct NodeData {
    pub(crate) data: Vec<Node>,
}

#[derive(Debug, Deserialize)]
pub struct Qemu {
    netin: u64,
    cpus: u32,
    netout: u64,
    status: String,
    name: String,
    maxmem: u64,
    disk: u64,
    uptime: u64,
    pub(crate) tags: Option<String>,
    pid: Option<u32>,
    cpu: f64,
    diskwrite: u64,
    diskread: u64,
    maxdisk: u64,
    mem: u64,
    pub(crate) vmid: u32,
}

#[derive(Debug, Deserialize)]
pub struct QemuData {
    pub(crate) data: Vec<Qemu>,
}

#[derive(Debug, Deserialize)]
pub struct NetworkInterfaceData {
    pub(crate) data: NetworkInterfaceResult
}

#[derive(Debug, Deserialize)]
pub struct NetworkInterfaceResult {
    pub(crate) result: Vec<NetworkInterface>
}

#[derive(Debug, Deserialize)]
pub struct NetworkInterface {
    #[serde(rename = "hardware-address")]
    hardware_address: String,
    pub(crate) name: String,
    #[serde(rename = "ip-addresses")]
    pub(crate) ip_addresses: Vec<IpAddress>,
}

#[derive(Debug, Deserialize)]
pub struct IpAddress {
    #[serde(rename = "ip-address-type")]
    pub(crate) ip_address_type: String,
    #[serde(rename = "ip-address")]
    pub(crate) ip_address: String,
    prefix: u32,
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