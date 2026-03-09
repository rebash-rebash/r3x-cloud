use crate::cloud::provider::*;
use std::collections::HashMap;

/// Scan all GCP forwarding rules (load balancers) using aggregatedList.
pub async fn scan_forwarding_rules(
    provider: &super::provider::GcpProvider,
    progress_tx: &tokio::sync::mpsc::Sender<ScanProgress>,
) -> anyhow::Result<Vec<CloudResource>> {
    let token = provider.get_access_token().await?;

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "LoadBalancer".to_string(),
            found: 0,
            status: ScanStepStatus::Scanning,
        })
        .await;

    let url = format!(
        "https://compute.googleapis.com/compute/v1/projects/{}/aggregated/forwardingRules",
        provider.project_id
    );

    let resp = provider.client.get(&url).bearer_auth(&token).send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        let _ = progress_tx
            .send(ScanProgress {
                account_id: provider.project_id.clone(),
                resource_type: "LoadBalancer".to_string(),
                found: 0,
                status: ScanStepStatus::Failed,
            })
            .await;
        return Err(anyhow::anyhow!("Failed to list forwarding rules: {} {}", status, body));
    }

    let data: serde_json::Value = resp.json().await?;
    let mut resources = Vec::new();

    if let Some(items) = data["items"].as_object() {
        for (region_key, region_data) in items {
            let region = region_key.strip_prefix("regions/").unwrap_or(region_key);
            if let Some(rules) = region_data["forwardingRules"].as_array() {
                for rule in rules {
                    if let Some(resource) = parse_forwarding_rule(rule, region, &provider.project_id) {
                        resources.push(resource);
                    }
                }
            }
        }
    }

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "LoadBalancer".to_string(),
            found: resources.len(),
            status: ScanStepStatus::Completed,
        })
        .await;

    Ok(resources)
}

fn parse_forwarding_rule(
    rule: &serde_json::Value,
    region: &str,
    project_id: &str,
) -> Option<CloudResource> {
    let id = rule["id"]
        .as_str()
        .map(String::from)
        .or_else(|| rule["id"].as_u64().map(|n| n.to_string()))?;

    let name = rule["name"].as_str()?.to_string();
    let ip_address = rule["IPAddress"].as_str().unwrap_or("");
    let protocol = rule["IPProtocol"].as_str().unwrap_or("");
    let port_range = rule["portRange"].as_str().unwrap_or("");
    let target = rule["target"].as_str().unwrap_or("");
    let target_short = target.rsplit('/').next().unwrap_or(target);
    let load_balancing_scheme = rule["loadBalancingScheme"].as_str().unwrap_or("EXTERNAL");

    let has_target = !target.is_empty();
    let status = if has_target { "ACTIVE" } else { "NO_TARGET" }.to_string();

    let created_at = rule["creationTimestamp"].as_str().map(String::from);

    let mut tags = HashMap::new();
    tags.insert("protocol".to_string(), protocol.to_string());
    tags.insert("scheme".to_string(), load_balancing_scheme.to_string());
    if let Some(labels) = rule["labels"].as_object() {
        for (k, v) in labels {
            if let Some(val) = v.as_str() {
                tags.insert(k.clone(), val.to_string());
            }
        }
    }

    let metadata = serde_json::json!({
        "ip_address": ip_address,
        "protocol": protocol,
        "port_range": port_range,
        "target": target_short,
        "load_balancing_scheme": load_balancing_scheme,
        "self_link": rule["selfLink"],
        "network": rule["network"],
        "subnetwork": rule["subnetwork"],
        "backend_service": rule["backendService"],
    });

    // Forwarding rule cost: ~$18/month base
    let cost = Some(18.26);

    Some(CloudResource {
        id,
        name,
        resource_type: ResourceType::LoadBalancer,
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
