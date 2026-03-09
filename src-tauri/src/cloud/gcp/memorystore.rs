use crate::cloud::provider::*;
use std::collections::HashMap;

/// Scan all Memorystore Redis instances in the project.
pub async fn scan_memorystore_instances(
    provider: &super::provider::GcpProvider,
    progress_tx: &tokio::sync::mpsc::Sender<ScanProgress>,
) -> anyhow::Result<Vec<CloudResource>> {
    let token = provider.get_access_token().await?;

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "MemorystoreInstance".to_string(),
            found: 0,
            status: ScanStepStatus::Scanning,
        })
        .await;

    let url = format!(
        "https://redis.googleapis.com/v1/projects/{}/locations/-/instances",
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

        // Redis API might not be enabled
        if status.as_u16() == 403 || status.as_u16() == 404 {
            let _ = progress_tx
                .send(ScanProgress {
                    account_id: provider.project_id.clone(),
                    resource_type: "MemorystoreInstance".to_string(),
                    found: 0,
                    status: ScanStepStatus::Completed,
                })
                .await;
            return Ok(vec![]);
        }

        let _ = progress_tx
            .send(ScanProgress {
                account_id: provider.project_id.clone(),
                resource_type: "MemorystoreInstance".to_string(),
                found: 0,
                status: ScanStepStatus::Failed,
            })
            .await;

        return Err(anyhow::anyhow!("Failed to list Memorystore instances: {} {}", status, body));
    }

    let data: serde_json::Value = resp.json().await?;
    let mut resources = Vec::new();

    if let Some(instances) = data["instances"].as_array() {
        for instance in instances {
            if let Some(resource) = parse_memorystore_instance(instance, &provider.project_id) {
                resources.push(resource);
            }
        }
    }

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "MemorystoreInstance".to_string(),
            found: resources.len(),
            status: ScanStepStatus::Completed,
        })
        .await;

    Ok(resources)
}

fn parse_memorystore_instance(instance: &serde_json::Value, project_id: &str) -> Option<CloudResource> {
    let full_name = instance["name"].as_str()?.to_string();
    // name is like "projects/{project}/locations/{location}/instances/{id}"
    let name = full_name.rsplit('/').next().unwrap_or(&full_name).to_string();
    let location = instance["locationId"].as_str().unwrap_or("unknown").to_string();
    let state = instance["state"].as_str().unwrap_or("UNKNOWN").to_string();
    let tier = instance["tier"].as_str().unwrap_or("BASIC");
    let memory_size_gb = instance["memorySizeGb"].as_f64()
        .or_else(|| instance["memorySizeGb"].as_i64().map(|v| v as f64))
        .unwrap_or(0.0);
    let redis_version = instance["redisVersion"].as_str().unwrap_or("unknown");
    let host = instance["host"].as_str().unwrap_or("unknown");
    let port = instance["port"].as_u64().unwrap_or(6379);
    let created_at = instance["createTime"].as_str().map(String::from);

    let mut tags = HashMap::new();
    if let Some(labels) = instance["labels"].as_object() {
        for (k, v) in labels {
            if let Some(val) = v.as_str() {
                tags.insert(k.clone(), val.to_string());
            }
        }
    }

    let metadata = serde_json::json!({
        "full_name": full_name,
        "tier": tier,
        "memory_size_gb": memory_size_gb,
        "redis_version": redis_version,
        "host": host,
        "port": port,
        "state": state,
    });

    let status = match state.as_str() {
        "READY" => "RUNNING".to_string(),
        "CREATING" => "CREATING".to_string(),
        "UPDATING" => "UPDATING".to_string(),
        "DELETING" => "DELETING".to_string(),
        "REPAIRING" => "REPAIRING".to_string(),
        other => other.to_string(),
    };

    let cost = estimate_memorystore_cost(tier, memory_size_gb);

    Some(CloudResource {
        id: name.clone(),
        name,
        resource_type: ResourceType::MemorystoreInstance,
        provider: ProviderKind::Gcp,
        region: location,
        account_id: project_id.to_string(),
        status,
        created_at,
        last_used: None,
        tags,
        metadata,
        monthly_cost: cost,
    })
}

/// Rough cost estimate for Memorystore Redis instances.
/// Basic tier: ~$0.049/GB/hour = ~$35.28/GB/month
/// Standard HA tier: ~$0.098/GB/hour = ~$70.56/GB/month
fn estimate_memorystore_cost(tier: &str, memory_size_gb: f64) -> Option<f64> {
    let per_gb_monthly = match tier {
        "STANDARD_HA" => 70.56,
        _ => 35.28, // BASIC or unknown
    };

    Some(memory_size_gb * per_gb_monthly)
}
