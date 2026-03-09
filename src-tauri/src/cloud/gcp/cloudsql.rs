use crate::cloud::provider::*;
use std::collections::HashMap;

/// Scan all Cloud SQL instances in the project.
pub async fn scan_sql_instances(
    provider: &super::provider::GcpProvider,
    progress_tx: &tokio::sync::mpsc::Sender<ScanProgress>,
) -> anyhow::Result<Vec<CloudResource>> {
    let token = provider.get_access_token().await?;

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "CloudSqlInstance".to_string(),
            found: 0,
            status: ScanStepStatus::Scanning,
        })
        .await;

    let url = format!(
        "https://sqladmin.googleapis.com/v1/projects/{}/instances",
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

        // Cloud SQL API might not be enabled
        if status.as_u16() == 403 || status.as_u16() == 404 {
            let _ = progress_tx
                .send(ScanProgress {
                    account_id: provider.project_id.clone(),
                    resource_type: "CloudSqlInstance".to_string(),
                    found: 0,
                    status: ScanStepStatus::Completed,
                })
                .await;
            return Ok(vec![]);
        }

        let _ = progress_tx
            .send(ScanProgress {
                account_id: provider.project_id.clone(),
                resource_type: "CloudSqlInstance".to_string(),
                found: 0,
                status: ScanStepStatus::Failed,
            })
            .await;

        return Err(anyhow::anyhow!("Failed to list SQL instances: {} {}", status, body));
    }

    let data: serde_json::Value = resp.json().await?;
    let mut resources = Vec::new();

    if let Some(items) = data["items"].as_array() {
        for instance in items {
            if let Some(resource) = parse_sql_instance(instance, &provider.project_id) {
                resources.push(resource);
            }
        }
    }

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "CloudSqlInstance".to_string(),
            found: resources.len(),
            status: ScanStepStatus::Completed,
        })
        .await;

    Ok(resources)
}

fn parse_sql_instance(instance: &serde_json::Value, project_id: &str) -> Option<CloudResource> {
    let name = instance["name"].as_str()?.to_string();
    let region = instance["region"].as_str().unwrap_or("unknown").to_string();
    let state = instance["state"].as_str().unwrap_or("UNKNOWN").to_string();
    let database_version = instance["databaseVersion"].as_str().unwrap_or("unknown");
    let tier = instance["settings"]["tier"].as_str().unwrap_or("unknown");
    let created_at = instance["createTime"].as_str().map(String::from);

    let data_disk_size_gb = instance["settings"]["dataDiskSizeGb"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| instance["settings"]["dataDiskSizeGb"].as_f64())
        .unwrap_or(0.0);

    let data_disk_type = instance["settings"]["dataDiskType"].as_str().unwrap_or("PD_SSD");
    let availability_type = instance["settings"]["availabilityType"].as_str().unwrap_or("ZONAL");
    let backup_enabled = instance["settings"]["backupConfiguration"]["enabled"].as_bool().unwrap_or(false);

    let ip_addresses: Vec<String> = instance["ipAddresses"]
        .as_array()
        .map(|addrs| {
            addrs.iter()
                .filter_map(|a| a["ipAddress"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let mut tags = HashMap::new();
    if let Some(labels) = instance["settings"]["userLabels"].as_object() {
        for (k, v) in labels {
            if let Some(val) = v.as_str() {
                tags.insert(k.clone(), val.to_string());
            }
        }
    }

    let metadata = serde_json::json!({
        "database_version": database_version,
        "tier": tier,
        "data_disk_size_gb": data_disk_size_gb,
        "data_disk_type": data_disk_type,
        "availability_type": availability_type,
        "backup_enabled": backup_enabled,
        "ip_addresses": ip_addresses,
        "instance_type": instance["instanceType"],
        "self_link": instance["selfLink"],
    });

    let cost = estimate_sql_cost(tier, data_disk_size_gb, availability_type);

    Some(CloudResource {
        id: name.clone(),
        name,
        resource_type: ResourceType::CloudSqlInstance,
        provider: ProviderKind::Gcp,
        region,
        account_id: project_id.to_string(),
        status: state,
        created_at,
        last_used: None,
        tags,
        metadata,
        monthly_cost: cost,
    })
}

/// Rough cost estimate for Cloud SQL instances (on-demand, us-central1).
fn estimate_sql_cost(tier: &str, disk_size_gb: f64, availability_type: &str) -> Option<f64> {
    let ha_multiplier = if availability_type == "REGIONAL" { 2.0 } else { 1.0 };

    let compute_cost = match tier {
        "db-f1-micro" => 7.67,
        "db-g1-small" => 25.55,
        "db-n1-standard-1" | "db-custom-1-3840" => 51.10,
        "db-n1-standard-2" | "db-custom-2-7680" => 102.20,
        "db-n1-standard-4" | "db-custom-4-15360" => 204.40,
        "db-n1-standard-8" | "db-custom-8-30720" => 408.80,
        "db-n1-standard-16" | "db-custom-16-61440" => 817.60,
        "db-n1-highmem-2" => 138.70,
        "db-n1-highmem-4" => 277.40,
        "db-n1-highmem-8" => 554.80,
        _ => 51.10, // default to small
    };

    // SSD storage: ~$0.170/GB/month
    let storage_cost = disk_size_gb * 0.170;

    Some((compute_cost * ha_multiplier) + storage_cost)
}
