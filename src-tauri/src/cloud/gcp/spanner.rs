use crate::cloud::provider::*;
use std::collections::HashMap;

/// Scan all Spanner instances in the project.
pub async fn scan_spanner_instances(
    provider: &super::provider::GcpProvider,
    progress_tx: &tokio::sync::mpsc::Sender<ScanProgress>,
) -> anyhow::Result<Vec<CloudResource>> {
    let token = provider.get_access_token().await?;

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "SpannerInstance".to_string(),
            found: 0,
            status: ScanStepStatus::Scanning,
        })
        .await;

    let url = format!(
        "https://spanner.googleapis.com/v1/projects/{}/instances",
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

        // Spanner API might not be enabled
        if status.as_u16() == 403 || status.as_u16() == 404 {
            let _ = progress_tx
                .send(ScanProgress {
                    account_id: provider.project_id.clone(),
                    resource_type: "SpannerInstance".to_string(),
                    found: 0,
                    status: ScanStepStatus::Completed,
                })
                .await;
            return Ok(vec![]);
        }

        let _ = progress_tx
            .send(ScanProgress {
                account_id: provider.project_id.clone(),
                resource_type: "SpannerInstance".to_string(),
                found: 0,
                status: ScanStepStatus::Failed,
            })
            .await;

        return Err(anyhow::anyhow!(
            "Failed to list Spanner instances: {} {}",
            status,
            body
        ));
    }

    let data: serde_json::Value = resp.json().await?;
    let mut resources = Vec::new();

    if let Some(instances) = data["instances"].as_array() {
        for instance in instances {
            if let Some(resource) = parse_spanner_instance(instance, &provider.project_id) {
                resources.push(resource);
            }
        }
    }

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "SpannerInstance".to_string(),
            found: resources.len(),
            status: ScanStepStatus::Completed,
        })
        .await;

    Ok(resources)
}

fn parse_spanner_instance(
    instance: &serde_json::Value,
    project_id: &str,
) -> Option<CloudResource> {
    let full_name = instance["name"].as_str()?.to_string();
    // Instance name format: projects/{project}/instances/{instance_name}
    let short_name = full_name
        .rsplit('/')
        .next()
        .unwrap_or(&full_name)
        .to_string();

    let display_name = instance["displayName"]
        .as_str()
        .unwrap_or(&short_name)
        .to_string();
    let config = instance["config"].as_str().unwrap_or("unknown").to_string();
    let state = instance["state"].as_str().unwrap_or("UNKNOWN").to_string();

    let node_count = instance["nodeCount"].as_u64().unwrap_or(0);
    let processing_units = instance["processingUnits"].as_u64().unwrap_or(0);

    // Determine if multi-region based on config name
    // Config format: projects/{project}/instanceConfigs/{config_name}
    // Multi-region configs typically contain "multi" in the name
    let is_multi_region = config.contains("multi") || config.contains("nam-eur") || config.contains("nam6");

    // Determine region from config
    let region = config
        .rsplit('/')
        .next()
        .unwrap_or("unknown")
        .to_string();

    // Map state to status
    let status = match state.as_str() {
        "READY" => "READY".to_string(),
        "CREATING" => "CREATING".to_string(),
        _ => state.clone(),
    };

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
        "display_name": display_name,
        "config": config,
        "node_count": node_count,
        "processing_units": processing_units,
        "state": state,
        "is_multi_region": is_multi_region,
        "endpoint_uris": instance["endpointUris"],
    });

    let cost = estimate_spanner_cost(node_count, processing_units, is_multi_region);

    Some(CloudResource {
        id: full_name.clone(),
        name: display_name,
        resource_type: ResourceType::SpannerInstance,
        provider: ProviderKind::Gcp,
        region,
        account_id: project_id.to_string(),
        status,
        created_at: None,
        last_used: None,
        tags,
        metadata,
        monthly_cost: cost,
    })
}

/// Rough cost estimate for Spanner instances.
/// Regional: $0.90/node/hour = ~$657/node/month
/// Multi-region: $2.70/node/hour = ~$1,971/node/month
/// If processingUnits are used, divide by 1000 for node equivalent.
fn estimate_spanner_cost(
    node_count: u64,
    processing_units: u64,
    is_multi_region: bool,
) -> Option<f64> {
    // Determine effective node count
    let effective_nodes = if node_count > 0 {
        node_count as f64
    } else if processing_units > 0 {
        processing_units as f64 / 1000.0
    } else {
        return Some(0.0);
    };

    let cost_per_node_per_month = if is_multi_region {
        // $2.70/node/hour * 730 hours/month
        2.70 * 730.0 // ~$1,971
    } else {
        // $0.90/node/hour * 730 hours/month
        0.90 * 730.0 // ~$657
    };

    Some(effective_nodes * cost_per_node_per_month)
}
