use crate::models::WebService;
use anyhow::Result;
use reqwest::Client;
use std::time::Duration;
use futures::future::join_all;

pub struct WebScanner {
    client: Client,
    services: Vec<WebServiceConfig>,
}

#[derive(Debug, Clone)]
pub struct WebServiceConfig {
    pub name: String,
    pub url: String,
}

impl WebScanner {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");

        let services = vec![
            WebServiceConfig {
                name: "Coolify".to_string(),
                url: "https://coolify.secure-penguin.com".to_string(),
            },
            WebServiceConfig {
                name: "Guacamole".to_string(),
                url: "https://guacamole.secure-penguin.com".to_string(),
            },
            WebServiceConfig {
                name: "N8n".to_string(),
                url: "https://n8n.secure-penguin.com".to_string(),
            },
            WebServiceConfig {
                name: "Obsidian".to_string(),
                url: "https://obsidian.secure-penguin.com".to_string(),
            },
            WebServiceConfig {
                name: "S3 Console".to_string(),
                url: "https://s3-console.secure-penguin.com".to_string(),
            },
            WebServiceConfig {
                name: "Traefik".to_string(),
                url: "https://traefik.secure-penguin.com".to_string(),
            },
        ];

        Self { client, services }
    }

    pub async fn scan_all(&self) -> Result<Vec<WebService>> {
        let scan_futures: Vec<_> = self
            .services
            .iter()
            .map(|config| self.scan_service(config.clone()))
            .collect();

        let results = join_all(scan_futures).await;
        
        let mut web_services = Vec::new();
        for result in results {
            match result {
                Ok(service) => web_services.push(service),
                Err(e) => {
                    eprintln!("Error scanning web service: {}", e);
                }
            }
        }

        Ok(web_services)
    }

    async fn scan_service(&self, config: WebServiceConfig) -> Result<WebService> {
        let start = std::time::Instant::now();
        
        let response = self.client
            .head(&config.url)
            .send()
            .await;

        let response_time = start.elapsed().as_secs_f64();

        match response {
            Ok(resp) => Ok(WebService {
                name: config.name.clone(),
                url: config.url.clone(),
                http_status: Some(resp.status().as_u16()),
                response_time: Some(response_time),
                error: None,
            }),
            Err(e) => Ok(WebService {
                name: config.name.clone(),
                url: config.url.clone(),
                http_status: None,
                response_time: Some(response_time),
                error: Some(e.to_string()),
            }),
        }
    }
}
