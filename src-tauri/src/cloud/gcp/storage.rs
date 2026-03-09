use crate::cloud::provider::*;
use std::collections::HashMap;

/// Scan all GCS buckets in the project.
pub async fn scan_buckets(
    provider: &super::provider::GcpProvider,
    progress_tx: &tokio::sync::mpsc::Sender<ScanProgress>,
) -> anyhow::Result<Vec<CloudResource>> {
    let token = provider.get_access_token().await?;

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "StorageBucket".to_string(),
            found: 0,
            status: ScanStepStatus::Scanning,
        })
        .await;

    let url = format!(
        "https://storage.googleapis.com/storage/v1/b?project={}",
        provider.project_id
    );

    let resp = provider
        .client
        .get(&url)
        .bearer_auth(&token)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();

        let _ = progress_tx
            .send(ScanProgress {
                account_id: provider.project_id.clone(),
                resource_type: "StorageBucket".to_string(),
                found: 0,
                status: ScanStepStatus::Failed,
            })
            .await;

        return Err(anyhow::anyhow!("Failed to list buckets: {} {}", status, body));
    }

    let data: serde_json::Value = resp.json().await?;
    let mut resources = Vec::new();

    if let Some(items) = data["items"].as_array() {
        for bucket in items {
            if let Some(resource) = parse_bucket(bucket, &provider.project_id) {
                resources.push(resource);
            }
        }
    }

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "StorageBucket".to_string(),
            found: resources.len(),
            status: ScanStepStatus::Completed,
        })
        .await;

    Ok(resources)
}

fn parse_bucket(bucket: &serde_json::Value, project_id: &str) -> Option<CloudResource> {
    let name = bucket["name"].as_str()?.to_string();
    let id = bucket["id"].as_str().unwrap_or(&name).to_string();
    let location = bucket["location"].as_str().unwrap_or("unknown").to_string();
    let storage_class = bucket["storageClass"].as_str().unwrap_or("STANDARD");
    let created_at = bucket["timeCreated"].as_str().map(String::from);
    let updated = bucket["updated"].as_str().unwrap_or("");

    let versioning_enabled = bucket["versioning"]["enabled"].as_bool().unwrap_or(false);

    let mut tags = HashMap::new();
    if let Some(labels) = bucket["labels"].as_object() {
        for (k, v) in labels {
            if let Some(val) = v.as_str() {
                tags.insert(k.clone(), val.to_string());
            }
        }
    }

    let metadata = serde_json::json!({
        "storage_class": storage_class,
        "location": &location,
        "location_type": bucket["locationType"],
        "versioning": versioning_enabled,
        "updated": updated,
        "default_event_based_hold": bucket["defaultEventBasedHold"],
        "self_link": bucket["selfLink"],
    });

    // Cost estimate: rough average for STANDARD class, varies by actual usage
    let cost = estimate_bucket_cost(storage_class);

    Some(CloudResource {
        id,
        name,
        resource_type: ResourceType::StorageBucket,
        provider: ProviderKind::Gcp,
        region: location.to_lowercase(),
        account_id: project_id.to_string(),
        status: "ACTIVE".to_string(),
        created_at,
        last_used: None,
        tags,
        metadata,
        monthly_cost: cost,
    })
}

/// Rough cost estimate per bucket (without knowing actual data size, we estimate minimal).
fn estimate_bucket_cost(storage_class: &str) -> Option<f64> {
    // Base cost without knowing size — just the minimum per-bucket overhead
    match storage_class {
        "STANDARD" => Some(0.0),       // $0.020/GB/month, but we don't know size
        "NEARLINE" => Some(0.0),       // $0.010/GB/month
        "COLDLINE" => Some(0.0),       // $0.004/GB/month
        "ARCHIVE" => Some(0.0),        // $0.0012/GB/month
        _ => Some(0.0),
    }
}
