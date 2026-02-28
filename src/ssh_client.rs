use crate::models::{VmHost, Service, ServiceStatus, Container, WireGuardStatus, WireGuardPeer, Port, LogEntry};
use anyhow::Result;
use std::process::Command;

pub struct SshClient {
    host: VmHost,
}

impl SshClient {
    pub async fn connect(host: VmHost) -> Result<Self> {
        let result = Command::new("ssh")
            .args([
                "-o", "StrictHostKeyChecking=no",
                "-o", "ConnectTimeout=10",
                "-o", "ServerAliveInterval=60",
                "-o", "ServerAliveCountMax=3",
                "-i", &host.identity_file,
                "-p", &host.port.to_string(),
                &format!("{}@{}", host.user, host.ip),
                "true"
            ])
            .output();

        match result {
            Ok(output) => {
                if output.status.success() {
                    return Ok(Self { host });
                }
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("SSH authentication failed: {}", stderr)
            }
            Err(e) => anyhow::bail!("Failed to execute SSH: {}", e),
        }
    }

    pub fn hostname(&self) -> Result<String> {
        self.run_command("hostname")
    }

    pub fn uptime(&self) -> Result<String> {
        self.run_command("uptime")
    }

    pub fn list_running_services(&self) -> Result<Vec<Service>> {
        let output = self.run_command("systemctl list-units --type=service --state=running --no-legend --plain")?;
        
        let mut services = Vec::new();
        let service_patterns = vec![
            "docker", "podman", "wireguard", "samba", "guacamole",
            "nginx", "traefik", "apache", "mysql", "postgres", "redis",
            "pdns", "powerdns", "n8n", "obsidian", "couchdb", "authelia"
        ];

        for line in output.lines() {
            let line = line.trim();
            if !line.is_empty() {
                for pattern in &service_patterns {
                    if line.to_lowercase().contains(pattern) {
                        services.push(Service {
                            name: line.to_string(),
                            status: ServiceStatus::Running,
                            ports: Vec::new(),
                        });
                    }
                }
            }
        }

        Ok(services)
    }

    pub fn list_containers(&self) -> Result<Vec<Container>> {
        if let Ok(output) = self.run_command("command -v docker >/dev/null 2>&1 && echo 'DOCKER_FOUND'") {
            if output.contains("DOCKER_FOUND") {
                return self.list_docker_containers();
            }
        }

        self.list_podman_containers()
    }

    fn list_docker_containers(&self) -> Result<Vec<Container>> {
        let output = self.run_command("sudo docker ps -a --format table name,status,ports 2>/dev/null || echo 'DOCKER_ERROR'")?;
        
        if output.contains("DOCKER_ERROR") || output.trim().is_empty() {
            return Ok(Vec::new());
        }

        let mut containers = Vec::new();
        for line in output.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                containers.push(Container {
                    name: parts[0].to_string(),
                    status: parts[1].to_string(),
                    ports: parts[2].to_string(),
                });
            }
        }

        Ok(containers)
    }

    fn list_podman_containers(&self) -> Result<Vec<Container>> {
        let output = self.run_command("sudo podman ps -a --format table name,status,ports 2>/dev/null || echo 'PODMAN_ERROR'")?;
        
        if output.contains("PODMAN_ERROR") || output.trim().is_empty() {
            return Ok(Vec::new());
        }

        let mut containers = Vec::new();
        for line in output.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                containers.push(Container {
                    name: parts[0].to_string(),
                    status: parts[1].to_string(),
                    ports: parts[2].to_string(),
                });
            }
        }

        Ok(containers)
    }

    pub fn get_wireguard_status(&self) -> Result<Option<WireGuardStatus>> {
        let output = self.run_command("sudo wg show 2>/dev/null || echo 'WG_ERROR'")?;

        if output.contains("WG_ERROR") || output.trim().is_empty() {
            return Ok(None);
        }

        let mut peers = Vec::new();
        let mut current_peer: Option<WireGuardPeer> = None;
        let mut public_key = String::new();
        let mut listening_port = 0u16;
        let mut interface = "wg0".to_string();

        for line in output.lines() {
            let line = line.trim();
            
            if line.starts_with("interface:") {
                interface = line.split(':').nth(1).unwrap_or("wg0").trim().to_string();
                if let Some(peer) = current_peer.take() {
                    peers.push(peer);
                }
            } else if line.starts_with("public key:") {
                public_key = line.split(':').nth(1).unwrap_or("unknown").trim().to_string();
            } else if line.starts_with("listening port:") {
                if let Some(port_str) = line.split(':').nth(1) {
                    listening_port = port_str.trim().parse::<u16>().unwrap_or(0);
                }
            } else if line.starts_with("peer:") {
                if let Some(peer) = current_peer.take() {
                    peers.push(peer);
                }
                current_peer = Some(WireGuardPeer {
                    public_key: line.split(':').nth(1).unwrap_or("unknown").trim().to_string(),
                    endpoint: None,
                    allowed_ips: String::new(),
                    latest_handshake: None,
                    transfer: None,
                });
            } else if line.starts_with("  endpoint:") {
                if let Some(ref mut peer) = current_peer {
                    peer.endpoint = Some(line.split(':').nth(1).unwrap_or("unknown").trim().to_string());
                }
            } else if line.starts_with("  allowed ips:") {
                if let Some(ref mut peer) = current_peer {
                    peer.allowed_ips = line.split(':').nth(1).unwrap_or("unknown").trim().to_string();
                }
            } else if line.starts_with("  latest handshake:") {
                if let Some(ref mut peer) = current_peer {
                    peer.latest_handshake = Some(line.split(':').nth(1).unwrap_or("unknown").trim().to_string());
                }
            } else if line.starts_with("  transfer:") {
                if let Some(ref mut peer) = current_peer {
                    peer.transfer = Some(line.split(':').nth(1).unwrap_or("unknown").trim().to_string());
                }
            }
        }

        if let Some(peer) = current_peer {
            peers.push(peer);
        }

        Ok(Some(WireGuardStatus {
            interface,
            public_key,
            listening_port,
            peers,
            error: None,
        }))
    }

    pub fn get_open_ports(&self) -> Result<Vec<Port>> {
        let output = self.run_command("ss -tulpn | grep LISTEN | head -20")?;
        
        let mut ports = Vec::new();
        for line in output.lines() {
            if let Some(port_str) = line.split(':').nth(1) {
                if let Some(port_part) = port_str.split_whitespace().next() {
                    if let Ok(port) = port_part.parse::<u16>() {
                        let protocol = line.split_whitespace().next()
                            .unwrap_or("unknown")
                            .to_string();
                        
                        let process = line
                            .split("users:(\"")
                            .nth(1)
                            .and_then(|s| s.split('"').next())
                            .unwrap_or("unknown")
                            .to_string();

                        ports.push(Port { port, protocol, process });
                    }
                }
            }
        }

        Ok(ports)
    }

    pub fn get_recent_errors(&self) -> Result<Vec<LogEntry>> {
        let output = self.run_command("journalctl --since '24 hours ago' --priority err --no-pager | tail -50 2>/dev/null || echo 'JOURNALCTL_ERROR'")?;

        if output.contains("JOURNALCTL_ERROR") || output.trim().is_empty() {
            return Ok(Vec::new());
        }

        let mut errors = Vec::new();
        for line in output.lines() {
            let parts: Vec<&str> = line.splitn(4, ' ').collect();
            if parts.len() >= 4 {
                errors.push(LogEntry {
                    timestamp: parts[0].to_string(),
                    service: parts.get(1).unwrap_or(&"unknown").to_string(),
                    level: "err".to_string(),
                    message: parts[2..].join(" "),
                });
            }
        }

        Ok(errors)
    }

    fn run_command(&self, command: &str) -> Result<String> {
        let result = Command::new("ssh")
            .args([
                "-o", "StrictHostKeyChecking=no",
                "-o", "ConnectTimeout=30",
                "-o", "ServerAliveInterval=60",
                "-i", &self.host.identity_file,
                "-p", &self.host.port.to_string(),
                &format!("{}@{}", self.host.user, self.host.ip),
                command,
            ])
            .output();

        match result {
            Ok(output) => {
                if output.status.success() {
                    Ok(String::from_utf8_lossy(&output.stdout).to_string())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    anyhow::bail!("Command failed: {}", stderr)
                }
            }
            Err(e) => anyhow::bail!("Failed to execute SSH command: {}", e),
        }
    }

    pub fn is_reachable(&self) -> bool {
        self.hostname().is_ok()
    }
}
