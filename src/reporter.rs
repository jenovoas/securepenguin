use crate::models::*;
use anyhow::{Context, Result};
use colored::Colorize;
use std::fs::File;
use std::io::Write;

pub struct MarkdownReporter;

impl MarkdownReporter {
    pub fn generate_report(report: &InventoryReport) -> Result<String> {
        let mut output = String::new();

        output.push_str(&Self::header(report));
        output.push_str(&Self::summary(&report.summary));
        output.push_str("\n## ESTADO POR VM\n\n");

        for vm in &report.vms {
            output.push_str(&Self::vm_status(vm));
            output.push_str("\n");
        }

        output.push_str("## SERVICIOS WEB EXTERNOS\n\n");
        output.push_str(&Self::web_services_table(&report.web_services));

        output.push_str("\n## ISSUES CRÍTICOS\n\n");
        if report.critical_issues.is_empty() {
            output.push_str("✅ No issues críticos encontrados\n");
        } else {
            for issue in &report.critical_issues {
                output.push_str(&format!("- ❌ {}\n", issue));
            }
        }

        output.push_str("\n## WARNINGS\n\n");
        if report.warnings.is_empty() {
            output.push_str("✅ No warnings\n");
        } else {
            for warning in &report.warnings {
                output.push_str(&format!("- ⚠️ {}\n", warning));
            }
        }

        output.push_str("\n---\n");
        output.push_str(&format!("*Generado por securepenguin-inventory*\n"));
        output.push_str(&format!(
            "*Fecha: {}*\n",
            report.timestamp.format("%Y-%m-%d %H:%M UTC")
        ));

        Ok(output)
    }

    fn header(report: &InventoryReport) -> String {
        format!(
            "# INVENTARIO STATUS SECUREPENGUIN\nFecha: {}\nHora: {}\n",
            report.timestamp.format("%Y-%m-%d"),
            report.timestamp.format("%H:%M UTC")
        )
    }

    fn summary(summary: &Summary) -> String {
        format!(
            "## RESUMEN EJECUTIVO\n\
            - VMs auditadas: {}/{}\n\
            - VMs accesibles: {}\n\
            - Servicios totales: {}\n\
            - Servicios corriendo: {}\n\
            - Servicios con problemas: {}\n\
            - Contenedores totales: {}\n\
            - Contenedores activos: {}\n",
            summary.reachable_vms,
            summary.total_vms,
            summary.reachable_vms,
            summary.total_services,
            summary.running_services,
            summary.failed_services,
            summary.total_containers,
            summary.running_containers,
        )
    }

    fn vm_status(vm: &VmStatus) -> String {
        let status_emoji = if vm.reachable { "✅" } else { "❌" };

        let mut output = format!(
            "### {} ({}:{})\n\
            **Estado:** {} {}\n\
            **Rol:** {}\n\n",
            vm.host.name,
            vm.host.ip,
            vm.host.port,
            status_emoji,
            if vm.reachable {
                "Operativa"
            } else {
                "Inaccesible"
            },
            vm.host.name
        );

        if vm.reachable {
            output.push_str("**Servicios:**\n");
            if vm.services.is_empty() {
                output.push_str("- Ninguno detectado\n");
            } else {
                for service in &vm.services {
                    let status_icon = match service.status {
                        ServiceStatus::Running => "✅",
                        ServiceStatus::Stopped => "⏸️",
                        ServiceStatus::Failed => "❌",
                        ServiceStatus::NotFound => "❓",
                    };
                    output.push_str(&format!(
                        "- {} {} (puertos: {:?})\n",
                        status_icon, service.name, service.ports
                    ));
                }
            }

            if !vm.containers.is_empty() {
                output.push_str("\n**Contenedores:**\n");
                for container in &vm.containers {
                    let status_emoji = if container.status.contains("Up") {
                        "✅"
                    } else {
                        "⏸️"
                    };
                    output.push_str(&format!(
                        "- {} {} {} - {}\n",
                        status_emoji, container.name, container.status, container.ports
                    ));
                }
            }

            if let Some(ref wg) = vm.wireguard {
                output.push_str(&format!(
                    "\n**WireGuard:**\n\
                    - Interface: {}\n\
                    - Public Key: {}\n\
                    - Listening Port: {}\n\
                    - Peers conectados: {}\n",
                    wg.interface,
                    wg.public_key,
                    wg.listening_port,
                    wg.peers.len()
                ));
            }

            if !vm.recent_errors.is_empty() {
                output.push_str("\n**Logs recientes (últimas 24h):**\n");
                for error in vm.recent_errors.iter().take(10) {
                    output.push_str(&format!(
                        "```\n{} {} {}\n```\n",
                        error.timestamp, error.service, error.message
                    ));
                }
            }
        }

        output
    }

    fn web_services_table(services: &[WebService]) -> String {
        let mut table = String::from("| Servicio | URL | HTTP Status | Tiempo response |\n");
        table.push_str("|----------|-----|-------------|----------------|\n");

        for service in services {
            let status = if let Some(status) = service.http_status {
                if (200..300).contains(&status) {
                    format!("{} {}", "✅", status)
                } else if (300..400).contains(&status) {
                    format!("{} {}", "⚠️", status)
                } else {
                    format!("{} {}", "❌", status)
                }
            } else if service.error.is_some() {
                format!("{} ERROR", "❌")
            } else {
                "?".to_string()
            };

            let time = service
                .response_time
                .map(|t| format!("{:.3}s", t))
                .unwrap_or_else(|| "N/A".to_string());

            table.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                service.name, service.url, status, time
            ));
        }

        table
    }

    pub fn save_report(report: &InventoryReport, output_path: &str) -> Result<()> {
        let markdown = Self::generate_report(report)?;
        let mut file = File::create(output_path)
            .context(format!("Failed to create report file: {}", output_path))?;

        file.write_all(markdown.as_bytes())
            .context("Failed to write report")?;

        println!("\n✅ Reporte guardado en: {}", output_path.green().bold());
        Ok(())
    }
}
