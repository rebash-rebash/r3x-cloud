use crate::cloud::provider::*;
use std::collections::HashMap;

/// Scan all Dataproc clusters in the project (across all regions).
pub async fn scan_dataproc_clusters(
    provider: &super::provider::GcpProvider,
    progress_tx: &tokio::sync::mpsc::Sender<ScanProgress>,
) -> anyhow::Result<Vec<CloudResource>> {
    let token = provider.get_access_token().await?;

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "DataprocCluster".to_string(),
            found: 0,
            status: ScanStepStatus::Scanning,
        })
        .await;

    let url = format!(
        "https://dataproc.googleapis.com/v1/projects/{}/regions/-/clusters",
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

        // Dataproc API might not be enabled
        if status.as_u16() == 403 || status.as_u16() == 404 {
            let _ = progress_tx
                .send(ScanProgress {
                    account_id: provider.project_id.clone(),
                    resource_type: "DataprocCluster".to_string(),
                    found: 0,
                    status: ScanStepStatus::Completed,
                })
                .await;
            return Ok(vec![]);
        }

        let _ = progress_tx
            .send(ScanProgress {
                account_id: provider.project_id.clone(),
                resource_type: "DataprocCluster".to_string(),
                found: 0,
                status: ScanStepStatus::Failed,
            })
            .await;

        return Err(anyhow::anyhow!("Failed to list Dataproc clusters: {} {}", status, body));
    }

    let data: serde_json::Value = resp.json().await?;
    let mut resources = Vec::new();

    if let Some(clusters) = data["clusters"].as_array() {
        for cluster in clusters {
            if let Some(resource) = parse_dataproc_cluster(cluster, &provider.project_id) {
                resources.push(resource);
            }
        }
    }

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "DataprocCluster".to_string(),
            found: resources.len(),
            status: ScanStepStatus::Completed,
        })
        .await;

    Ok(resources)
}

fn parse_dataproc_cluster(cluster: &serde_json::Value, project_id: &str) -> Option<CloudResource> {
    let name = cluster["clusterName"].as_str()?.to_string();
    let state = cluster["status"]["state"]
        .as_str()
        .unwrap_or("UNKNOWN")
        .to_string();

    // Extract region from zoneUri (e.g. ".../zones/us-central1-a" -> "us-central1")
    let region = cluster["config"]["gceClusterConfig"]["zoneUri"]
        .as_str()
        .and_then(|z| {
            // Zone URI looks like: projects/proj/zones/us-central1-a
            let zone = z.rsplit('/').next()?;
            // Strip the last "-X" suffix to get region
            let last_dash = zone.rfind('-')?;
            Some(zone[..last_dash].to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    let master_num_instances = cluster["config"]["masterConfig"]["numInstances"]
        .as_u64()
        .or_else(|| {
            cluster["config"]["masterConfig"]["numInstances"]
                .as_str()
                .and_then(|s| s.parse().ok())
        })
        .unwrap_or(0);

    let master_machine_type = cluster["config"]["masterConfig"]["machineTypeUri"]
        .as_str()
        .and_then(|uri| uri.rsplit('/').next())
        .unwrap_or("unknown");

    let worker_num_instances = cluster["config"]["workerConfig"]["numInstances"]
        .as_u64()
        .or_else(|| {
            cluster["config"]["workerConfig"]["numInstances"]
                .as_str()
                .and_then(|s| s.parse().ok())
        })
        .unwrap_or(0);

    let worker_machine_type = cluster["config"]["workerConfig"]["machineTypeUri"]
        .as_str()
        .and_then(|uri| uri.rsplit('/').next())
        .unwrap_or("unknown");

    let mut tags = HashMap::new();
    if let Some(labels) = cluster["labels"].as_object() {
        for (k, v) in labels {
            if let Some(val) = v.as_str() {
                tags.insert(k.clone(), val.to_string());
            }
        }
    }

    let metadata = serde_json::json!({
        "master_num_instances": master_num_instances,
        "master_machine_type": master_machine_type,
        "worker_num_instances": worker_num_instances,
        "worker_machine_type": worker_machine_type,
        "status_history": cluster["statusHistory"],
        "config_bucket": cluster["config"]["configBucket"],
        "software_config": cluster["config"]["softwareConfig"],
    });

    let cost = estimate_dataproc_cost(
        master_num_instances,
        master_machine_type,
        worker_num_instances,
        worker_machine_type,
    );

    Some(CloudResource {
        id: name.clone(),
        name,
        resource_type: ResourceType::DataprocCluster,
        provider: ProviderKind::Gcp,
        region,
        account_id: project_id.to_string(),
        status: state,
        created_at: None,
        last_used: None,
        tags,
        metadata,
        monthly_cost: cost,
    })
}

/// Rough cost estimate for Dataproc clusters.
/// Base Compute Engine cost + Dataproc premium ($0.01/vCPU/hour).
fn estimate_dataproc_cost(
    master_nodes: u64,
    master_type: &str,
    worker_nodes: u64,
    worker_type: &str,
) -> Option<f64> {
    let compute_cost_per_node = |machine_type: &str| -> f64 {
        match machine_type {
            "e2-standard-2" | "n1-standard-2" => 50.0,
            "e2-standard-4" | "n1-standard-4" => 100.0,
            "e2-standard-8" | "n1-standard-8" => 200.0,
            "e2-standard-16" | "n1-standard-16" => 400.0,
            "e2-standard-32" | "n1-standard-32" => 800.0,
            "e2-highmem-2" | "n1-highmem-2" => 68.0,
            "e2-highmem-4" | "n1-highmem-4" => 136.0,
            "e2-highmem-8" | "n1-highmem-8" => 272.0,
            "e2-highcpu-2" | "n1-highcpu-2" => 37.0,
            "e2-highcpu-4" | "n1-highcpu-4" => 74.0,
            "e2-highcpu-8" | "n1-highcpu-8" => 148.0,
            _ => 50.0, // default baseline
        }
    };

    let vcpu_count = |machine_type: &str| -> f64 {
        // Extract vCPU count from machine type name (last number)
        machine_type
            .rsplit('-')
            .next()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(2.0)
    };

    let hours_per_month = 730.0;
    let dataproc_premium_per_vcpu_hour = 0.01;

    let master_compute = master_nodes as f64 * compute_cost_per_node(master_type);
    let worker_compute = worker_nodes as f64 * compute_cost_per_node(worker_type);

    let master_vcpus = master_nodes as f64 * vcpu_count(master_type);
    let worker_vcpus = worker_nodes as f64 * vcpu_count(worker_type);
    let dataproc_premium = (master_vcpus + worker_vcpus) * dataproc_premium_per_vcpu_hour * hours_per_month;

    Some(master_compute + worker_compute + dataproc_premium)
}
