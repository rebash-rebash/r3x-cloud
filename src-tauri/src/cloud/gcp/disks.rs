use crate::cloud::provider::*;
use std::collections::HashMap;

/// Scan all GCP persistent disks using aggregatedList.
pub async fn scan_disks(
    provider: &super::provider::GcpProvider,
    progress_tx: &tokio::sync::mpsc::Sender<ScanProgress>,
) -> anyhow::Result<Vec<CloudResource>> {
    let token = provider.get_access_token().await?;

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "Disk".to_string(),
            found: 0,
            status: ScanStepStatus::Scanning,
        })
        .await;

    let url = format!(
        "https://compute.googleapis.com/compute/v1/projects/{}/aggregated/disks",
        provider.project_id
    );

    let resp = provider.client.get(&url).bearer_auth(&token).send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        let _ = progress_tx
            .send(ScanProgress {
                account_id: provider.project_id.clone(),
                resource_type: "Disk".to_string(),
                found: 0,
                status: ScanStepStatus::Failed,
            })
            .await;
        return Err(anyhow::anyhow!("Failed to list disks: {} {}", status, body));
    }

    let data: serde_json::Value = resp.json().await?;
    let mut resources = Vec::new();

    if let Some(items) = data["items"].as_object() {
        for (zone_key, zone_data) in items {
            let zone = zone_key.strip_prefix("zones/").unwrap_or(zone_key);
            if let Some(disks) = zone_data["disks"].as_array() {
                for disk in disks {
                    if let Some(resource) = parse_disk(disk, zone, &provider.project_id) {
                        resources.push(resource);
                    }
                }
            }
        }
    }

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "Disk".to_string(),
            found: resources.len(),
            status: ScanStepStatus::Completed,
        })
        .await;

    Ok(resources)
}

fn parse_disk(disk: &serde_json::Value, zone: &str, project_id: &str) -> Option<CloudResource> {
    let id = disk["id"]
        .as_str()
        .map(String::from)
        .or_else(|| disk["id"].as_u64().map(|n| n.to_string()))?;

    let name = disk["name"].as_str()?.to_string();
    let size_gb = disk["sizeGb"].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
    let disk_type = disk["type"].as_str().unwrap_or("");
    let disk_type_short = disk_type.rsplit('/').next().unwrap_or(disk_type);
    let users = disk["users"].as_array();
    let attached = users.map_or(false, |u| !u.is_empty());
    let status = if attached { "ATTACHED" } else { "UNATTACHED" }.to_string();

    let created_at = disk["creationTimestamp"].as_str().map(String::from);

    let mut tags = HashMap::new();
    if let Some(labels) = disk["labels"].as_object() {
        for (k, v) in labels {
            if let Some(val) = v.as_str() {
                tags.insert(k.clone(), val.to_string());
            }
        }
    }

    let metadata = serde_json::json!({
        "disk_type": disk_type_short,
        "size_gb": size_gb,
        "zone": zone,
        "self_link": disk["selfLink"],
        "users": disk["users"],
        "source_image": disk["sourceImage"],
        "source_snapshot": disk["sourceSnapshot"],
        "physical_block_size_bytes": disk["physicalBlockSizeBytes"],
    });

    Some(CloudResource {
        id,
        name,
        resource_type: ResourceType::Disk,
        provider: ProviderKind::Gcp,
        region: zone.to_string(),
        account_id: project_id.to_string(),
        status,
        created_at,
        last_used: None,
        tags,
        metadata,
        monthly_cost: estimate_disk_cost(disk_type_short, size_gb),
    })
}

/// Estimate monthly cost for GCP disks (USD/GB/month).
fn estimate_disk_cost(disk_type: &str, size_gb: f64) -> Option<f64> {
    let per_gb = match disk_type {
        "pd-standard" => 0.04,
        "pd-balanced" => 0.10,
        "pd-ssd" => 0.17,
        "pd-extreme" => 0.125,
        "hyperdisk-balanced" => 0.06,
        _ => return None,
    };
    Some(per_gb * size_gb)
}
