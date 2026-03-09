use crate::cloud::provider::*;
use std::collections::HashMap;

/// Scan all custom GCP images (global resource). Excludes public images.
pub async fn scan_images(
    provider: &super::provider::GcpProvider,
    progress_tx: &tokio::sync::mpsc::Sender<ScanProgress>,
) -> anyhow::Result<Vec<CloudResource>> {
    let token = provider.get_access_token().await?;

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "MachineImage".to_string(),
            found: 0,
            status: ScanStepStatus::Scanning,
        })
        .await;

    let url = format!(
        "https://compute.googleapis.com/compute/v1/projects/{}/global/images",
        provider.project_id
    );

    let resp = provider.client.get(&url).bearer_auth(&token).send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        let _ = progress_tx
            .send(ScanProgress {
                account_id: provider.project_id.clone(),
                resource_type: "MachineImage".to_string(),
                found: 0,
                status: ScanStepStatus::Failed,
            })
            .await;
        return Err(anyhow::anyhow!("Failed to list images: {} {}", status, body));
    }

    let data: serde_json::Value = resp.json().await?;
    let mut resources = Vec::new();

    if let Some(items) = data["items"].as_array() {
        for img in items {
            if let Some(resource) = parse_image(img, &provider.project_id) {
                resources.push(resource);
            }
        }
    }

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "MachineImage".to_string(),
            found: resources.len(),
            status: ScanStepStatus::Completed,
        })
        .await;

    Ok(resources)
}

fn parse_image(img: &serde_json::Value, project_id: &str) -> Option<CloudResource> {
    let id = img["id"]
        .as_str()
        .map(String::from)
        .or_else(|| img["id"].as_u64().map(|n| n.to_string()))?;

    let name = img["name"].as_str()?.to_string();
    let status = img["status"].as_str().unwrap_or("UNKNOWN").to_string();
    let created_at = img["creationTimestamp"].as_str().map(String::from);
    let disk_size_gb = img["diskSizeGb"].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
    let archive_size_bytes = img["archiveSizeBytes"].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
    let deprecated = img["deprecated"]["state"].as_str().unwrap_or("");

    let mut tags = HashMap::new();
    if !deprecated.is_empty() {
        tags.insert("deprecated".to_string(), deprecated.to_string());
    }
    if let Some(labels) = img["labels"].as_object() {
        for (k, v) in labels {
            if let Some(val) = v.as_str() {
                tags.insert(k.clone(), val.to_string());
            }
        }
    }

    let metadata = serde_json::json!({
        "disk_size_gb": disk_size_gb,
        "archive_size_bytes": archive_size_bytes,
        "self_link": img["selfLink"],
        "source_disk": img["sourceDisk"],
        "source_type": img["sourceType"],
        "family": img["family"],
        "deprecated": img["deprecated"],
        "storage_locations": img["storageLocations"],
    });

    // Image storage cost: ~$0.05/GB/month
    let storage_gb = archive_size_bytes / (1024.0 * 1024.0 * 1024.0);
    let cost = if storage_gb > 0.0 { Some(storage_gb * 0.05) } else { None };

    Some(CloudResource {
        id,
        name,
        resource_type: ResourceType::MachineImage,
        provider: ProviderKind::Gcp,
        region: "global".to_string(),
        account_id: project_id.to_string(),
        status,
        created_at,
        last_used: None,
        tags,
        metadata,
        monthly_cost: cost,
    })
}
