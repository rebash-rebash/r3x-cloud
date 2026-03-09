use crate::cloud::provider::*;
use std::collections::HashMap;

/// Scan all GCP disk snapshots (global resource, not per-zone).
pub async fn scan_snapshots(
    provider: &super::provider::GcpProvider,
    progress_tx: &tokio::sync::mpsc::Sender<ScanProgress>,
) -> anyhow::Result<Vec<CloudResource>> {
    let token = provider.get_access_token().await?;

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "Snapshot".to_string(),
            found: 0,
            status: ScanStepStatus::Scanning,
        })
        .await;

    let url = format!(
        "https://compute.googleapis.com/compute/v1/projects/{}/global/snapshots",
        provider.project_id
    );

    let resp = provider.client.get(&url).bearer_auth(&token).send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        let _ = progress_tx
            .send(ScanProgress {
                account_id: provider.project_id.clone(),
                resource_type: "Snapshot".to_string(),
                found: 0,
                status: ScanStepStatus::Failed,
            })
            .await;
        return Err(anyhow::anyhow!("Failed to list snapshots: {} {}", status, body));
    }

    let data: serde_json::Value = resp.json().await?;
    let mut resources = Vec::new();

    if let Some(items) = data["items"].as_array() {
        for snap in items {
            if let Some(resource) = parse_snapshot(snap, &provider.project_id) {
                resources.push(resource);
            }
        }
    }

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "Snapshot".to_string(),
            found: resources.len(),
            status: ScanStepStatus::Completed,
        })
        .await;

    Ok(resources)
}

fn parse_snapshot(snap: &serde_json::Value, project_id: &str) -> Option<CloudResource> {
    let id = snap["id"]
        .as_str()
        .map(String::from)
        .or_else(|| snap["id"].as_u64().map(|n| n.to_string()))?;

    let name = snap["name"].as_str()?.to_string();
    let status = snap["status"].as_str().unwrap_or("UNKNOWN").to_string();
    let created_at = snap["creationTimestamp"].as_str().map(String::from);
    let size_gb = snap["diskSizeGb"].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
    let storage_bytes = snap["storageBytes"].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);

    // Extract source disk location as "region"
    let source_disk = snap["sourceDisk"].as_str().unwrap_or("");
    let region = source_disk
        .split("/zones/")
        .nth(1)
        .and_then(|s| s.split('/').next())
        .unwrap_or("global")
        .to_string();

    let mut tags = HashMap::new();
    if let Some(labels) = snap["labels"].as_object() {
        for (k, v) in labels {
            if let Some(val) = v.as_str() {
                tags.insert(k.clone(), val.to_string());
            }
        }
    }

    let metadata = serde_json::json!({
        "disk_size_gb": size_gb,
        "storage_bytes": storage_bytes,
        "source_disk": source_disk,
        "self_link": snap["selfLink"],
        "storage_locations": snap["storageLocations"],
    });

    // Snapshot storage cost: ~$0.026/GB/month
    let storage_gb = storage_bytes / (1024.0 * 1024.0 * 1024.0);
    let cost = if storage_gb > 0.0 { Some(storage_gb * 0.026) } else { None };

    Some(CloudResource {
        id,
        name,
        resource_type: ResourceType::Snapshot,
        provider: ProviderKind::Gcp,
        region,
        account_id: project_id.to_string(),
        status,
        created_at,
        last_used: None,
        tags,
        metadata,
        monthly_cost: cost,
    })
}
