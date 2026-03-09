use crate::cloud::provider::*;
use std::collections::HashMap;

/// Scan all GKE clusters in the project.
pub async fn scan_gke_clusters(
    provider: &super::provider::GcpProvider,
    progress_tx: &tokio::sync::mpsc::Sender<ScanProgress>,
) -> anyhow::Result<Vec<CloudResource>> {
    let token = provider.get_access_token().await?;

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "GkeCluster".to_string(),
            found: 0,
            status: ScanStepStatus::Scanning,
        })
        .await;

    let url = format!(
        "https://container.googleapis.com/v1/projects/{}/locations/-/clusters",
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

        // GKE API might not be enabled
        if status.as_u16() == 403 || status.as_u16() == 404 {
            let _ = progress_tx
                .send(ScanProgress {
                    account_id: provider.project_id.clone(),
                    resource_type: "GkeCluster".to_string(),
                    found: 0,
                    status: ScanStepStatus::Completed,
                })
                .await;
            return Ok(vec![]);
        }

        let _ = progress_tx
            .send(ScanProgress {
                account_id: provider.project_id.clone(),
                resource_type: "GkeCluster".to_string(),
                found: 0,
                status: ScanStepStatus::Failed,
            })
            .await;

        return Err(anyhow::anyhow!("Failed to list GKE clusters: {} {}", status, body));
    }

    let data: serde_json::Value = resp.json().await?;
    let mut resources = Vec::new();

    if let Some(clusters) = data["clusters"].as_array() {
        for cluster in clusters {
            if let Some(resource) = parse_gke_cluster(cluster, &provider.project_id) {
                resources.push(resource);
            }
        }
    }

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "GkeCluster".to_string(),
            found: resources.len(),
            status: ScanStepStatus::Completed,
        })
        .await;

    Ok(resources)
}

fn parse_gke_cluster(cluster: &serde_json::Value, project_id: &str) -> Option<CloudResource> {
    let name = cluster["name"].as_str()?.to_string();
    let location = cluster["location"].as_str().unwrap_or("unknown").to_string();
    let gke_status = cluster["status"].as_str().unwrap_or("UNKNOWN");

    let current_node_count = cluster["currentNodeCount"]
        .as_u64()
        .or_else(|| cluster["currentNodeCount"].as_f64().map(|f| f as u64))
        .unwrap_or(0);
    let initial_node_count = cluster["initialNodeCount"]
        .as_u64()
        .or_else(|| cluster["initialNodeCount"].as_f64().map(|f| f as u64))
        .unwrap_or(0);

    let machine_type = cluster["nodeConfig"]["machineType"]
        .as_str()
        .unwrap_or("e2-medium")
        .to_string();
    let disk_size_gb = cluster["nodeConfig"]["diskSizeGb"]
        .as_u64()
        .or_else(|| cluster["nodeConfig"]["diskSizeGb"].as_f64().map(|f| f as u64))
        .unwrap_or(100);
    let master_version = cluster["currentMasterVersion"]
        .as_str()
        .unwrap_or("unknown");

    let created_at = cluster["createTime"].as_str().map(String::from);

    let status = match gke_status {
        "RUNNING" => "RUNNING".to_string(),
        "STOPPING" => "STOPPING".to_string(),
        "ERROR" => "ERROR".to_string(),
        "PROVISIONING" => "PROVISIONING".to_string(),
        "RECONCILING" => "RECONCILING".to_string(),
        "DEGRADED" => "DEGRADED".to_string(),
        other => other.to_string(),
    };

    let mut tags = HashMap::new();
    if let Some(labels) = cluster["resourceLabels"].as_object() {
        for (k, v) in labels {
            if let Some(val) = v.as_str() {
                tags.insert(k.clone(), val.to_string());
            }
        }
    }

    let metadata = serde_json::json!({
        "current_node_count": current_node_count,
        "initial_node_count": initial_node_count,
        "machine_type": machine_type,
        "disk_size_gb": disk_size_gb,
        "current_master_version": master_version,
        "network_config": cluster["networkConfig"],
        "autoscaling": cluster["autoscaling"],
        "node_config": cluster["nodeConfig"],
        "self_link": cluster["selfLink"],
    });

    let cost = estimate_gke_cost(&machine_type, current_node_count);

    Some(CloudResource {
        id: name.clone(),
        name,
        resource_type: ResourceType::GkeCluster,
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

/// Rough cost estimate for GKE clusters (on-demand, us-central1).
/// Based on node count * machine type cost, plus $72/month management fee per cluster.
fn estimate_gke_cost(machine_type: &str, node_count: u64) -> Option<f64> {
    let per_node_cost = match machine_type {
        "e2-medium" => 25.0,
        "e2-standard-2" => 50.0,
        "e2-standard-4" => 100.0,
        "e2-standard-8" => 200.0,
        "e2-standard-16" => 400.0,
        "n1-standard-1" => 25.0,
        "n1-standard-2" => 50.0,
        "n1-standard-4" => 100.0,
        "n1-standard-8" => 200.0,
        "n1-standard-16" => 400.0,
        "n2-standard-2" => 50.0,
        "n2-standard-4" => 100.0,
        "n2-standard-8" => 200.0,
        "n2-standard-16" => 400.0,
        _ => 50.0, // default per-node cost
    };

    let management_fee = 72.0;
    let total = (per_node_cost * node_count as f64) + management_fee;

    Some(total)
}
