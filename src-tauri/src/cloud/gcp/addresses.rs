use crate::cloud::provider::*;
use std::collections::HashMap;

/// Scan all GCP static external IPs using aggregatedList.
pub async fn scan_addresses(
    provider: &super::provider::GcpProvider,
    progress_tx: &tokio::sync::mpsc::Sender<ScanProgress>,
) -> anyhow::Result<Vec<CloudResource>> {
    let token = provider.get_access_token().await?;

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "ElasticIp".to_string(),
            found: 0,
            status: ScanStepStatus::Scanning,
        })
        .await;

    let url = format!(
        "https://compute.googleapis.com/compute/v1/projects/{}/aggregated/addresses",
        provider.project_id
    );

    let resp = provider.client.get(&url).bearer_auth(&token).send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        let _ = progress_tx
            .send(ScanProgress {
                account_id: provider.project_id.clone(),
                resource_type: "ElasticIp".to_string(),
                found: 0,
                status: ScanStepStatus::Failed,
            })
            .await;
        return Err(anyhow::anyhow!("Failed to list addresses: {} {}", status, body));
    }

    let data: serde_json::Value = resp.json().await?;
    let mut resources = Vec::new();

    if let Some(items) = data["items"].as_object() {
        for (region_key, region_data) in items {
            let region = region_key.strip_prefix("regions/").unwrap_or(region_key);
            if let Some(addresses) = region_data["addresses"].as_array() {
                for addr in addresses {
                    if let Some(resource) = parse_address(addr, region, &provider.project_id) {
                        resources.push(resource);
                    }
                }
            }
        }
    }

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "ElasticIp".to_string(),
            found: resources.len(),
            status: ScanStepStatus::Completed,
        })
        .await;

    Ok(resources)
}

fn parse_address(addr: &serde_json::Value, region: &str, project_id: &str) -> Option<CloudResource> {
    let id = addr["id"]
        .as_str()
        .map(String::from)
        .or_else(|| addr["id"].as_u64().map(|n| n.to_string()))?;

    let name = addr["name"].as_str()?.to_string();
    let address = addr["address"].as_str().unwrap_or("").to_string();
    // RESERVED = not in use, IN_USE = attached
    let status = addr["status"].as_str().unwrap_or("UNKNOWN").to_string();
    let created_at = addr["creationTimestamp"].as_str().map(String::from);
    let address_type = addr["addressType"].as_str().unwrap_or("EXTERNAL");

    let mut tags = HashMap::new();
    if let Some(labels) = addr["labels"].as_object() {
        for (k, v) in labels {
            if let Some(val) = v.as_str() {
                tags.insert(k.clone(), val.to_string());
            }
        }
    }

    let metadata = serde_json::json!({
        "address": address,
        "address_type": address_type,
        "region": region,
        "self_link": addr["selfLink"],
        "users": addr["users"],
        "network_tier": addr["networkTier"],
    });

    // Unused static IP costs ~$7.30/month
    let cost = if status == "RESERVED" { Some(7.30) } else { Some(0.0) };

    Some(CloudResource {
        id,
        name,
        resource_type: ResourceType::ElasticIp,
        provider: ProviderKind::Gcp,
        region: region.to_string(),
        account_id: project_id.to_string(),
        status,
        created_at,
        last_used: None,
        tags,
        metadata,
        monthly_cost: cost,
    })
}
