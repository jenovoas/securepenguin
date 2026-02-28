mod models;
mod ssh_client;
mod web_scanner;
mod scanner;
mod reporter;

use anyhow::{Context, Result};
use colored::*;
use models::VmHost;

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n{}", "╔══════════════════════════════════════════╗".cyan());
    println!("{}", "║  SECUREPENGUIN INVENTORY SCANNER           ║".cyan());
    println!("{}\n", "╚══════════════════════════════════════════╝".cyan());

    let hosts = load_ssh_config()?;
    
    println!("{} Loaded {} VMs from SSH config", 
        "[✓]".green().bold(), hosts.len());

    let inventory_scanner = scanner::InventoryScanner::new(hosts);
    
    println!("{} Starting inventory scan...", 
        "[→]".blue().bold());

    let report = inventory_scanner.scan()
        .await
        .context("Failed to complete inventory scan")?;

    let output_path = "/home/jnovoas/SecurePenguin/INVENTARIO_STATUS_AUTO.md";
    
    reporter::MarkdownReporter::save_report(&report, &output_path)?;

    print_summary(&report);

    Ok(())
}

fn load_ssh_config() -> Result<Vec<VmHost>> {
    // Parse ~/.ssh/config to extract VM hosts
    let ssh_config_path = "/home/jnovoas/.ssh/config";
    
    let config_content = std::fs::read_to_string(&ssh_config_path)
        .context("Failed to read SSH config")?;

    let mut hosts = Vec::new();
    let mut current_host: Option<VmHost> = None;

    for line in config_content.lines() {
        let line = line.trim();
        
        if line.starts_with("Host ") {
            // Save previous host if exists
            if let Some(host) = current_host.take() {
                hosts.push(host);
            }
            
            let name = line[5..].trim().to_string();
            // Filter out backup hosts
            if !name.ends_with("-bkp") && name != "pirex" {
                current_host = Some(VmHost {
                    name: name.clone(),
                    ip: String::new(),
                    port: 22,
                    user: String::new(),
                    identity_file: String::new(),
                    vpn_ip: None,
                });
            }
        } else if let Some(ref mut host) = current_host {
            if line.starts_with("HostName ") {
                host.ip = line[9..].trim().to_string();
            } else if line.starts_with("Port ") {
                host.port = line[5..].trim().parse().unwrap_or(22);
            } else if line.starts_with("User ") {
                host.user = line[5..].trim().to_string();
            } else if line.starts_with("IdentityFile ") {
                host.identity_file = line[12..].trim().to_string();
            }
        }
    }

    // Add pirex separately (different SSH config pattern)
    hosts.push(VmHost {
        name: "pirex".to_string(),
        ip: "34.176.56.176".to_string(),
        port: 22,
        user: "jnovoas".to_string(),
        identity_file: "/home/jnovoas/.ssh/id_oracle".to_string(),
        vpn_ip: Some("10.10.10.7".to_string()),
    });

    // Manually add kingu, sentinel, centurion VPN IPs
    hosts.iter_mut().for_each(|host| {
        host.vpn_ip = match host.name.as_str() {
            "kingu" => Some("10.10.10.1".to_string()),
            "sentinel" => Some("10.10.10.2".to_string()),
            "centurion" => Some("10.10.10.3".to_string()),
            _ => None,
        };
    });

    Ok(hosts)
}

fn print_summary(report: &models::InventoryReport) {
    println!("\n{}", "══════════════════════════════════════════".cyan());
    println!("{}", "SCAN SUMMARY".cyan());
    println!("{}\n", "══════════════════════════════════════════".cyan());

    println!("VMs totales:        {}", report.summary.total_vms.to_string().white().bold());
    println!("VMs accesibles:     {}", report.summary.reachable_vms.to_string().green().bold());
    println!("Servicios corriendo: {}", report.summary.running_services.to_string().green().bold());
    println!("Contenedores activos: {}", report.summary.running_containers.to_string().green().bold());
    
    if !report.critical_issues.is_empty() {
        println!("\n{} Issues críticos: {}", 
            "❌".red().bold(), report.critical_issues.len());
        for issue in &report.critical_issues {
            println!("  - {}", issue.red());
        }
    }

    if !report.warnings.is_empty() {
        println!("\n{} Warnings: {}", 
            "⚠️".yellow().bold(), report.warnings.len());
        for warning in &report.warnings {
            println!("  - {}", warning.yellow());
        }
    }

    if report.critical_issues.is_empty() && report.warnings.is_empty() {
        println!("\n{}", "✅ Todos los sistemas operativos!".green().bold());
    }

    println!("\n{}", "══════════════════════════════════════════\n".cyan());
}
