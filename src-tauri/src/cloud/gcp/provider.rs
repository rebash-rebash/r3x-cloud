use crate::cloud::provider::*;
use async_trait::async_trait;
use std::path::PathBuf;

/// Find the gcloud CLI binary. GUI apps on macOS don't inherit shell PATH,
/// so we check common installation locations.
fn find_gcloud() -> Option<String> {
    // 1. Try PATH first (works in terminal / dev mode)
    if let Ok(output) = std::process::Command::new("which").arg("gcloud").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }

    // 2. Check common install locations
    let home = std::env::var("HOME").unwrap_or_default();
    let candidates = [
        format!("{}/google-cloud-sdk/bin/gcloud", home),
        format!("{}/Downloads/google-cloud-sdk/bin/gcloud", home),
        "/usr/local/bin/gcloud".to_string(),
        "/opt/homebrew/bin/gcloud".to_string(),
        "/usr/bin/gcloud".to_string(),
        "/snap/bin/gcloud".to_string(),
        format!("{}/snap/google-cloud-sdk/current/bin/gcloud", home),
        "/usr/local/Caskroom/google-cloud-sdk/latest/google-cloud-sdk/bin/gcloud".to_string(),
    ];

    for candidate in &candidates {
        if PathBuf::from(candidate).exists() {
            return Some(candidate.clone());
        }
    }

    None
}

pub struct GcpProvider {
    pub project_id: String,
    pub client: reqwest::Client,
    pub config: serde_json::Value,
}

impl GcpProvider {
    pub fn new(project_id: String, config: serde_json::Value) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        Ok(Self {
            project_id,
            client,
            config,
        })
    }

    /// Get an access token using gcloud CLI.
    /// Uses `gcloud auth print-access-token` which respects the active gcloud account,
    /// rather than ADC which requires separate quota project permissions.
    pub async fn get_access_token(&self) -> anyhow::Result<String> {
        let gcloud_path = find_gcloud().ok_or_else(|| {
            anyhow::anyhow!(
                "Could not find gcloud CLI. Ensure Google Cloud SDK is installed and you've run 'gcloud auth login'."
            )
        })?;

        let output = tokio::process::Command::new(&gcloud_path)
            .args(["auth", "print-access-token"])
            .output()
            .await
            .map_err(|e| anyhow::anyhow!(
                "Failed to run gcloud CLI at '{}'. Error: {}",
                gcloud_path, e
            ))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "gcloud auth failed: {}. Run 'gcloud auth login' first.",
                stderr.trim()
            ));
        }

        let token = String::from_utf8(output.stdout)?
            .trim()
            .to_string();

        if token.is_empty() {
            return Err(anyhow::anyhow!("Empty access token from gcloud CLI"));
        }

        Ok(token)
    }
}

#[async_trait]
impl CloudProvider for GcpProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Gcp
    }

    fn account_id(&self) -> &str {
        &self.project_id
    }

    async fn validate_credentials(&self) -> anyhow::Result<String> {
        let token = self.get_access_token().await?;

        // Validate token by checking tokeninfo endpoint (no project permissions needed)
        let resp = self
            .client
            .get("https://oauth2.googleapis.com/tokeninfo")
            .query(&[("access_token", &token)])
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Invalid access token. Run 'gcloud auth login' first."));
        }

        let info: serde_json::Value = resp.json().await?;
        let email = info["email"].as_str().unwrap_or("unknown");

        Ok(format!(
            "Authenticated as {} for project: {}",
            email, self.project_id
        ))
    }

    async fn list_regions(&self) -> anyhow::Result<Vec<String>> {
        let token = self.get_access_token().await?;

        let url = format!(
            "https://compute.googleapis.com/compute/v1/projects/{}/zones",
            self.project_id
        );

        let resp = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Failed to list zones: {}", body));
        }

        let data: serde_json::Value = resp.json().await?;
        let zones = data["items"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item["name"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(zones)
    }

    async fn scan_resource_type(
        &self,
        _region: &str,
        resource_type: &ResourceType,
        progress_tx: tokio::sync::mpsc::Sender<ScanProgress>,
    ) -> anyhow::Result<Vec<CloudResource>> {
        match resource_type {
            ResourceType::VirtualMachine => {
                super::compute::scan_instances(self, &progress_tx).await
            }
            ResourceType::Disk => {
                super::disks::scan_disks(self, &progress_tx).await
            }
            ResourceType::Snapshot => {
                super::snapshots::scan_snapshots(self, &progress_tx).await
            }
            ResourceType::ElasticIp => {
                super::addresses::scan_addresses(self, &progress_tx).await
            }
            ResourceType::SecurityGroup => {
                super::firewalls::scan_firewalls(self, &progress_tx).await
            }
            ResourceType::LoadBalancer => {
                super::forwarding_rules::scan_forwarding_rules(self, &progress_tx).await
            }
            ResourceType::MachineImage => {
                super::images::scan_images(self, &progress_tx).await
            }
            _ => {
                tracing::warn!(
                    "Resource type {} not yet implemented for GCP",
                    resource_type
                );
                Ok(vec![])
            }
        }
    }

    fn supported_resource_types(&self) -> Vec<ResourceType> {
        vec![
            ResourceType::VirtualMachine,
            ResourceType::Disk,
            ResourceType::Snapshot,
            ResourceType::ElasticIp,
            ResourceType::SecurityGroup,
            ResourceType::LoadBalancer,
            ResourceType::MachineImage,
        ]
    }
}
