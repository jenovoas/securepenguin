use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmHost {
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub user: String,
    pub identity_file: String,
    pub vpn_ip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmStatus {
    pub host: VmHost,
    pub reachable: bool,
    pub services: Vec<Service>,
    pub containers: Vec<Container>,
    pub wireguard: Option<WireGuardStatus>,
    pub open_ports: Vec<Port>,
    pub recent_errors: Vec<LogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub name: String,
    pub status: ServiceStatus,
    pub ports: Vec<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServiceStatus {
    Running,
    Stopped,
    Failed,
    NotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub name: String,
    pub status: String,
    pub ports: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireGuardStatus {
    pub interface: String,
    pub public_key: String,
    pub listening_port: u16,
    pub peers: Vec<WireGuardPeer>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireGuardPeer {
    pub public_key: String,
    pub endpoint: Option<String>,
    pub allowed_ips: String,
    pub latest_handshake: Option<String>,
    pub transfer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Port {
    pub port: u16,
    pub protocol: String,
    pub process: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub service: String,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebService {
    pub name: String,
    pub url: String,
    pub http_status: Option<u16>,
    pub response_time: Option<f64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryReport {
    pub timestamp: DateTime<Utc>,
    pub vms: Vec<VmStatus>,
    pub web_services: Vec<WebService>,
    pub summary: Summary,
    pub critical_issues: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub total_vms: usize,
    pub reachable_vms: usize,
    pub total_services: usize,
    pub running_services: usize,
    pub failed_services: usize,
    pub total_containers: usize,
    pub running_containers: usize,
}
