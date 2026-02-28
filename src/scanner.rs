use crate::models::*;
use crate::ssh_client::SshClient;
use crate::web_scanner::WebScanner;
use anyhow::Result;
use chrono::Utc;
use colored::Colorize;

pub struct InventoryScanner {
    hosts: Vec<VmHost>,
}

impl InventoryScanner {
    pub fn new(hosts: Vec<VmHost>) -> Self {
        Self { hosts }
    }

    pub async fn scan(&self) -> Result<InventoryReport> {
        let web_scanner = WebScanner::new();
        let web_services = web_scanner.scan_all().await?;

        let mut vms = Vec::new();
        let mut critical_issues = Vec::new();
        let mut warnings = Vec::new();

        println!("{} Scanning VMs...", "[*]".blue().bold());

        for host in &self.hosts {
            println!("  Checking {}...", host.name.cyan());
            
            match SshClient::connect(host.clone()).await {
                Ok(ssh_client) => {
                    let reachable = ssh_client.is_reachable();
                    
                    if !reachable {
                        warnings.push(format!("{} is not reachable", host.name));
                    }

                    let services = ssh_client.list_running_services().unwrap_or_default();
                    let containers = ssh_client.list_containers().unwrap_or_default();
                    let wireguard = ssh_client.get_wireguard_status().unwrap_or(None);
                    let open_ports = ssh_client.get_open_ports().unwrap_or_default();
                    let recent_errors = ssh_client.get_recent_errors().unwrap_or_default();

                    // Check for critical issues
                    self.check_critical_issues(&host, &services, &recent_errors, &mut critical_issues);
                    
                    vms.push(VmStatus {
                        host: host.clone(),
                        reachable,
                        services,
                        containers,
                        wireguard,
                        open_ports,
                        recent_errors,
                    });
                }
                Err(e) => {
                    println!("    {} Failed: {}", "✗".red(), e);
                    critical_issues.push(format!("{}: {}", host.name, e));
                    
                    vms.push(VmStatus {
                        host: host.clone(),
                        reachable: false,
                        services: Vec::new(),
                        containers: Vec::new(),
                        wireguard: None,
                        open_ports: Vec::new(),
                        recent_errors: Vec::new(),
                    });
                }
            }
        }

        let summary = self.generate_summary(&vms);

        Ok(InventoryReport {
            timestamp: Utc::now(),
            vms,
            web_services,
            summary,
            critical_issues,
            warnings,
        })
    }

    fn check_critical_issues(
        &self,
        host: &VmHost,
        services: &[Service],
        errors: &[LogEntry],
        critical_issues: &mut Vec<String>,
    ) {
        // Check for port conflicts
        let mut port_usage: std::collections::HashMap<u16, Vec<&Service>> = std::collections::HashMap::new();
        
        for service in services {
            if matches!(service.status, ServiceStatus::Running) {
                for port in &service.ports {
                    port_usage.entry(*port).or_insert_with(Vec::new).push(service);
                }
            }
        }

        for (port, svc_list) in &port_usage {
            if svc_list.len() > 1 {
                let names: Vec<&str> = svc_list.iter().map(|s| s.name.as_str()).collect();
                critical_issues.push(format!(
                    "{}: Port conflict on {} - used by {:?}",
                    host.name, port, names
                ));
            }
        }

        // Check for specific errors
        for error in errors {
            if error.message.contains("NT_STATUS_ADDRESS_ALREADY_ASSOCIATED") 
                || error.message.contains("Failed to bind")
                || error.message.contains("port.*already")
            {
                critical_issues.push(format!(
                    "{}: Port binding error - {}",
                    host.name, error.message
                ));
            }
        }
    }

    fn generate_summary(&self, vms: &[VmStatus]) -> Summary {
        let total_vms = vms.len();
        let reachable_vms = vms.iter().filter(|v| v.reachable).count();
        
        let total_services: usize = vms.iter().map(|v| v.services.len()).sum();
        let running_services: usize = vms.iter()
            .map(|v| v.services.iter().filter(|s| matches!(s.status, ServiceStatus::Running)).count())
            .sum();
        
        let total_containers: usize = vms.iter().map(|v| v.containers.len()).sum();
        let running_containers: usize = vms.iter()
            .map(|v| v.containers.iter().filter(|c| c.status.contains("Up")).count())
            .sum();

        Summary {
            total_vms,
            reachable_vms,
            total_services,
            running_services,
            failed_services: total_services - running_services,
            total_containers,
            running_containers,
        }
    }
}
