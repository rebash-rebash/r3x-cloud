use crate::cloud::provider::*;
use std::collections::HashMap;

/// Scan all GCP firewall rules (global resource).
pub async fn scan_firewalls(
    provider: &super::provider::GcpProvider,
    progress_tx: &tokio::sync::mpsc::Sender<ScanProgress>,
) -> anyhow::Result<Vec<CloudResource>> {
    let token = provider.get_access_token().await?;

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "SecurityGroup".to_string(),
            found: 0,
            status: ScanStepStatus::Scanning,
        })
        .await;

    let url = format!(
        "https://compute.googleapis.com/compute/v1/projects/{}/global/firewalls",
        provider.project_id
    );

    let resp = provider.client.get(&url).bearer_auth(&token).send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        let _ = progress_tx
            .send(ScanProgress {
                account_id: provider.project_id.clone(),
                resource_type: "SecurityGroup".to_string(),
                found: 0,
                status: ScanStepStatus::Failed,
            })
            .await;
        return Err(anyhow::anyhow!("Failed to list firewalls: {} {}", status, body));
    }

    let data: serde_json::Value = resp.json().await?;
    let mut resources = Vec::new();

    if let Some(items) = data["items"].as_array() {
        for fw in items {
            if let Some(resource) = parse_firewall(fw, &provider.project_id) {
                resources.push(resource);
            }
        }
    }

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "SecurityGroup".to_string(),
            found: resources.len(),
            status: ScanStepStatus::Completed,
        })
        .await;

    Ok(resources)
}

fn parse_firewall(fw: &serde_json::Value, project_id: &str) -> Option<CloudResource> {
    let id = fw["id"]
        .as_str()
        .map(String::from)
        .or_else(|| fw["id"].as_u64().map(|n| n.to_string()))?;

    let name = fw["name"].as_str()?.to_string();
    let direction = fw["direction"].as_str().unwrap_or("INGRESS");
    let disabled = fw["disabled"].as_bool().unwrap_or(false);
    let status = if disabled { "DISABLED" } else { "ENABLED" }.to_string();
    let created_at = fw["creationTimestamp"].as_str().map(String::from);

    let network = fw["network"].as_str().unwrap_or("");
    let network_short = network.rsplit('/').next().unwrap_or(network);

    let target_tags = fw["targetTags"]
        .as_array()
        .map(|t| t.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
        .unwrap_or_default();

    let mut tags = HashMap::new();
    tags.insert("direction".to_string(), direction.to_string());
    tags.insert("network".to_string(), network_short.to_string());
    if !target_tags.is_empty() {
        tags.insert("target_tags".to_string(), target_tags);
    }

    let metadata = serde_json::json!({
        "direction": direction,
        "network": network_short,
        "priority": fw["priority"],
        "self_link": fw["selfLink"],
        "allowed": fw["allowed"],
        "denied": fw["denied"],
        "source_ranges": fw["sourceRanges"],
        "destination_ranges": fw["destinationRanges"],
        "target_tags": fw["targetTags"],
        "target_service_accounts": fw["targetServiceAccounts"],
    });

    Some(CloudResource {
        id,
        name,
        resource_type: ResourceType::SecurityGroup,
        provider: ProviderKind::Gcp,
        region: "global".to_string(),
        account_id: project_id.to_string(),
        status,
        created_at,
        last_used: None,
        tags,
        metadata,
        monthly_cost: Some(0.0), // Firewall rules are free
    })
}
