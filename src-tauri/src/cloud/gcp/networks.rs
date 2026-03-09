use crate::cloud::provider::*;
use std::collections::HashMap;

/// Scan all VPC networks in the project.
pub async fn scan_networks(
    provider: &super::provider::GcpProvider,
    progress_tx: &tokio::sync::mpsc::Sender<ScanProgress>,
) -> anyhow::Result<Vec<CloudResource>> {
    let token = provider.get_access_token().await?;

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "Network".to_string(),
            found: 0,
            status: ScanStepStatus::Scanning,
        })
        .await;

    let url = format!(
        "https://compute.googleapis.com/compute/v1/projects/{}/global/networks",
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
                resource_type: "Network".to_string(),
                found: 0,
                status: ScanStepStatus::Failed,
            })
            .await;

        return Err(anyhow::anyhow!("Failed to list networks: {} {}", status, body));
    }

    let data: serde_json::Value = resp.json().await?;
    let mut resources = Vec::new();

    if let Some(items) = data["items"].as_array() {
        for network in items {
            if let Some(resource) = parse_network(network, &provider.project_id) {
                resources.push(resource);
            }
        }
    }

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "Network".to_string(),
            found: resources.len(),
            status: ScanStepStatus::Completed,
        })
        .await;

    Ok(resources)
}

fn parse_network(network: &serde_json::Value, project_id: &str) -> Option<CloudResource> {
    let id = network["id"]
        .as_str()
        .map(String::from)
        .or_else(|| network["id"].as_u64().map(|n| n.to_string()))?;

    let name = network["name"].as_str()?.to_string();
    let created_at = network["creationTimestamp"].as_str().map(String::from);
    let auto_create_subnets = network["autoCreateSubnetworks"].as_bool().unwrap_or(false);
    let routing_mode = network["routingConfig"]["routingMode"].as_str().unwrap_or("REGIONAL");

    let subnet_count = network["subnetworks"]
        .as_array()
        .map(|s| s.len())
        .unwrap_or(0);

    let peering_count = network["peerings"]
        .as_array()
        .map(|p| p.len())
        .unwrap_or(0);

    let peerings: Vec<String> = network["peerings"]
        .as_array()
        .map(|p| {
            p.iter()
                .filter_map(|peer| peer["name"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let metadata = serde_json::json!({
        "auto_create_subnetworks": auto_create_subnets,
        "routing_mode": routing_mode,
        "subnet_count": subnet_count,
        "peering_count": peering_count,
        "peerings": peerings,
        "self_link": network["selfLink"],
        "mtu": network["mtu"],
    });

    // Networks are free, but associated resources (subnets, peerings) may cost
    Some(CloudResource {
        id,
        name,
        resource_type: ResourceType::Network,
        provider: ProviderKind::Gcp,
        region: "global".to_string(),
        account_id: project_id.to_string(),
        status: "ACTIVE".to_string(),
        created_at,
        last_used: None,
        tags: HashMap::new(), // VPC networks don't have labels in GCP
        metadata,
        monthly_cost: Some(0.0),
    })
}
