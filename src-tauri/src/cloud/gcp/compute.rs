use crate::cloud::provider::*;
use std::collections::HashMap;

/// Scan all GCP Compute Engine instances using aggregatedList (single API call for all zones).
pub async fn scan_instances(
    provider: &super::provider::GcpProvider,
    progress_tx: &tokio::sync::mpsc::Sender<ScanProgress>,
) -> anyhow::Result<Vec<CloudResource>> {
    let token = provider.get_access_token().await?;

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "VirtualMachine".to_string(),
            found: 0,
            status: ScanStepStatus::Scanning,
        })
        .await;

    // aggregatedList returns instances across ALL zones in one call
    let url = format!(
        "https://compute.googleapis.com/compute/v1/projects/{}/aggregated/instances",
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
                resource_type: "VirtualMachine".to_string(),
                found: 0,
                status: ScanStepStatus::Failed,
            })
            .await;

        return Err(anyhow::anyhow!(
            "Failed to list instances: {} {}",
            status,
            body
        ));
    }

    let data: serde_json::Value = resp.json().await?;
    let mut resources = Vec::new();

    // aggregatedList returns { "items": { "zones/us-central1-a": { "instances": [...] }, ... } }
    if let Some(items) = data["items"].as_object() {
        for (zone_key, zone_data) in items {
            let zone = zone_key.strip_prefix("zones/").unwrap_or(zone_key);

            if let Some(instances) = zone_data["instances"].as_array() {
                for instance in instances {
                    if let Some(resource) = parse_instance(instance, zone, &provider.project_id) {
                        resources.push(resource);
                    }
                }
            }
        }
    }

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "VirtualMachine".to_string(),
            found: resources.len(),
            status: ScanStepStatus::Completed,
        })
        .await;

    Ok(resources)
}

fn parse_instance(
    instance: &serde_json::Value,
    zone: &str,
    project_id: &str,
) -> Option<CloudResource> {
    let id = instance["id"]
        .as_str()
        .map(String::from)
        .or_else(|| instance["id"].as_u64().map(|n| n.to_string()))?;

    let name = instance["name"].as_str()?.to_string();
    let status = instance["status"].as_str().unwrap_or("UNKNOWN").to_string();
    let created_at = instance["creationTimestamp"].as_str().map(String::from);

    let machine_type = instance["machineType"].as_str().unwrap_or("");
    let machine_type_short = machine_type.rsplit('/').next().unwrap_or(machine_type);

    let mut tags = HashMap::new();
    if let Some(labels) = instance["labels"].as_object() {
        for (k, v) in labels {
            if let Some(val) = v.as_str() {
                tags.insert(k.clone(), val.to_string());
            }
        }
    }

    let metadata = serde_json::json!({
        "machine_type": machine_type_short,
        "zone": zone,
        "self_link": instance["selfLink"],
        "network_interfaces": instance["networkInterfaces"],
        "disks": instance["disks"],
        "can_ip_forward": instance["canIpForward"],
        "scheduling": instance["scheduling"],
    });

    Some(CloudResource {
        id,
        name,
        resource_type: ResourceType::VirtualMachine,
        provider: ProviderKind::Gcp,
        region: zone.to_string(),
        account_id: project_id.to_string(),
        status,
        created_at,
        last_used: None,
        tags,
        metadata,
        monthly_cost: estimate_vm_cost(machine_type_short),
    })
}

/// Rough cost estimate for common GCP machine types (USD/month, on-demand, us-central1).
fn estimate_vm_cost(machine_type: &str) -> Option<f64> {
    match machine_type {
        "e2-micro" => Some(7.67),
        "e2-small" => Some(15.33),
        "e2-medium" => Some(30.67),
        "e2-standard-2" => Some(61.34),
        "e2-standard-4" => Some(122.67),
        "e2-standard-8" => Some(245.35),
        "e2-standard-16" => Some(490.69),
        "e2-standard-32" => Some(981.38),
        "e2-highmem-2" => Some(82.77),
        "e2-highmem-4" => Some(165.54),
        "e2-highmem-8" => Some(331.09),
        "e2-highcpu-2" => Some(45.63),
        "e2-highcpu-4" => Some(91.25),
        "e2-highcpu-8" => Some(182.50),
        "n1-standard-1" => Some(34.67),
        "n1-standard-2" => Some(69.35),
        "n1-standard-4" => Some(138.70),
        "n1-standard-8" => Some(277.40),
        "n2-standard-2" => Some(71.54),
        "n2-standard-4" => Some(143.08),
        "n2-standard-8" => Some(286.16),
        "n2d-standard-2" => Some(62.19),
        "n2d-standard-4" => Some(124.38),
        "f1-micro" => Some(4.67),
        "g1-small" => Some(15.33),
        _ => None,
    }
}
