use crate::cloud::provider::*;
use std::collections::HashMap;

/// Scan all Cloud Run services across all regions.
pub async fn scan_services(
    provider: &super::provider::GcpProvider,
    progress_tx: &tokio::sync::mpsc::Sender<ScanProgress>,
) -> anyhow::Result<Vec<CloudResource>> {
    let token = provider.get_access_token().await?;

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "CloudRunService".to_string(),
            found: 0,
            status: ScanStepStatus::Scanning,
        })
        .await;

    // Use wildcard location "-" to list services across all regions
    let url = format!(
        "https://run.googleapis.com/v2/projects/{}/locations/-/services",
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

        // Cloud Run API might not be enabled
        if status.as_u16() == 403 || status.as_u16() == 404 {
            let _ = progress_tx
                .send(ScanProgress {
                    account_id: provider.project_id.clone(),
                    resource_type: "CloudRunService".to_string(),
                    found: 0,
                    status: ScanStepStatus::Completed,
                })
                .await;
            return Ok(vec![]);
        }

        let _ = progress_tx
            .send(ScanProgress {
                account_id: provider.project_id.clone(),
                resource_type: "CloudRunService".to_string(),
                found: 0,
                status: ScanStepStatus::Failed,
            })
            .await;

        return Err(anyhow::anyhow!("Failed to list Cloud Run services: {} {}", status, body));
    }

    let data: serde_json::Value = resp.json().await?;
    let mut resources = Vec::new();

    if let Some(services) = data["services"].as_array() {
        for svc in services {
            if let Some(resource) = parse_service(svc, &provider.project_id) {
                resources.push(resource);
            }
        }
    }

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "CloudRunService".to_string(),
            found: resources.len(),
            status: ScanStepStatus::Completed,
        })
        .await;

    Ok(resources)
}

fn parse_service(svc: &serde_json::Value, project_id: &str) -> Option<CloudResource> {
    // v2 name format: projects/{project}/locations/{location}/services/{name}
    let full_name = svc["name"].as_str()?;
    let parts: Vec<&str> = full_name.split('/').collect();
    let name = parts.last()?.to_string();
    let location = parts.get(3).unwrap_or(&"unknown").to_string();

    let create_time = svc["createTime"].as_str().map(String::from);
    let update_time = svc["updateTime"].as_str().map(String::from);
    let uri = svc["uri"].as_str().unwrap_or("");
    let ingress = svc["ingress"].as_str().unwrap_or("INGRESS_TRAFFIC_ALL");

    // Check conditions for status
    let conditions = svc["conditions"].as_array();
    let ready = conditions
        .and_then(|conds| {
            conds.iter().find(|c| c["type"].as_str() == Some("Ready"))
        })
        .and_then(|c| c["state"].as_str())
        .unwrap_or("UNKNOWN");

    let status = match ready {
        "CONDITION_SUCCEEDED" => "ACTIVE",
        "CONDITION_FAILED" => "FAILED",
        _ => "UNKNOWN",
    };

    // Template details (latest revision)
    let template = &svc["template"];
    let containers = template["containers"].as_array();
    let image = containers
        .and_then(|c| c.first())
        .and_then(|c| c["image"].as_str())
        .unwrap_or("unknown");
    let memory = containers
        .and_then(|c| c.first())
        .and_then(|c| c["resources"]["limits"]["memory"].as_str())
        .unwrap_or("512Mi");
    let cpu = containers
        .and_then(|c| c.first())
        .and_then(|c| c["resources"]["limits"]["cpu"].as_str())
        .unwrap_or("1");
    let max_instances = template["scaling"]["maxInstanceCount"].as_i64().unwrap_or(100);
    let min_instances = template["scaling"]["minInstanceCount"].as_i64().unwrap_or(0);

    let mut tags = HashMap::new();
    if let Some(labels) = svc["labels"].as_object() {
        for (k, v) in labels {
            if let Some(val) = v.as_str() {
                tags.insert(k.clone(), val.to_string());
            }
        }
    }

    let metadata = serde_json::json!({
        "image": image,
        "memory": memory,
        "cpu": cpu,
        "max_instances": max_instances,
        "min_instances": min_instances,
        "url": uri,
        "ingress": ingress,
        "latest_revision": svc["latestReadyRevision"],
    });

    // Cost: pay per use unless min_instances > 0
    let cost = if min_instances > 0 {
        Some(min_instances as f64 * 15.0) // rough idle cost per min instance
    } else {
        Some(0.0)
    };

    Some(CloudResource {
        id: full_name.to_string(),
        name,
        resource_type: ResourceType::CloudRunService,
        provider: ProviderKind::Gcp,
        region: location,
        account_id: project_id.to_string(),
        status: status.to_string(),
        created_at: create_time,
        last_used: update_time,
        tags,
        metadata,
        monthly_cost: cost,
    })
}
