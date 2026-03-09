use crate::cloud::provider::*;
use std::collections::HashMap;

/// Scan all Log Sinks (exports) in the project.
pub async fn scan_log_sinks(
    provider: &super::provider::GcpProvider,
    progress_tx: &tokio::sync::mpsc::Sender<ScanProgress>,
) -> anyhow::Result<Vec<CloudResource>> {
    let token = provider.get_access_token().await?;

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "LogSink".to_string(),
            found: 0,
            status: ScanStepStatus::Scanning,
        })
        .await;

    let url = format!(
        "https://logging.googleapis.com/v2/projects/{}/sinks",
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

        // Logging API might not be enabled or accessible
        if status.as_u16() == 403 || status.as_u16() == 404 {
            let _ = progress_tx
                .send(ScanProgress {
                    account_id: provider.project_id.clone(),
                    resource_type: "LogSink".to_string(),
                    found: 0,
                    status: ScanStepStatus::Completed,
                })
                .await;
            return Ok(vec![]);
        }

        let _ = progress_tx
            .send(ScanProgress {
                account_id: provider.project_id.clone(),
                resource_type: "LogSink".to_string(),
                found: 0,
                status: ScanStepStatus::Failed,
            })
            .await;

        return Err(anyhow::anyhow!("Failed to list log sinks: {} {}", status, body));
    }

    let data: serde_json::Value = resp.json().await?;
    let mut resources = Vec::new();

    if let Some(sinks) = data["sinks"].as_array() {
        for sink in sinks {
            if let Some(resource) = parse_log_sink(sink, &provider.project_id) {
                resources.push(resource);
            }
        }
    }

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "LogSink".to_string(),
            found: resources.len(),
            status: ScanStepStatus::Completed,
        })
        .await;

    Ok(resources)
}

fn parse_log_sink(sink: &serde_json::Value, project_id: &str) -> Option<CloudResource> {
    let name = sink["name"].as_str()?.to_string();

    let destination = sink["destination"].as_str().unwrap_or("unknown").to_string();
    let filter = sink["filter"].as_str().unwrap_or("").to_string();
    let disabled = sink["disabled"].as_bool().unwrap_or(false);
    let writer_identity = sink["writerIdentity"].as_str().unwrap_or("").to_string();
    let create_time = sink["createTime"].as_str().map(String::from);
    let update_time = sink["updateTime"].as_str().map(String::from);

    let status = if disabled {
        "DISABLED".to_string()
    } else {
        "ACTIVE".to_string()
    };

    // Classify destination type
    let destination_type = if destination.starts_with("bigquery.googleapis.com/") {
        "bigquery"
    } else if destination.starts_with("storage.googleapis.com/") {
        "cloud_storage"
    } else if destination.starts_with("pubsub.googleapis.com/") {
        "pubsub"
    } else if destination.starts_with("logging.googleapis.com/") {
        "logging_bucket"
    } else {
        "other"
    };

    let tags = HashMap::new();

    let metadata = serde_json::json!({
        "destination": destination,
        "destination_type": destination_type,
        "filter": filter,
        "disabled": disabled,
        "writer_identity": writer_identity,
        "update_time": update_time,
    });

    let cost = estimate_log_sink_cost(destination_type, &filter);

    Some(CloudResource {
        id: name.clone(),
        name,
        resource_type: ResourceType::LogSink,
        provider: ProviderKind::Gcp,
        region: "global".to_string(),
        account_id: project_id.to_string(),
        status,
        created_at: create_time,
        last_used: None,
        tags,
        metadata,
        monthly_cost: cost,
    })
}

/// Rough cost estimate for Log Sinks.
/// Logging ingestion: $0.50/GB after 50GB free tier.
/// Sinks route data to destinations with their own costs:
///   - BigQuery: ~$0.05/GB streaming insert + storage
///   - Cloud Storage: ~$0.02/GB storage
///   - Pub/Sub: ~$0.04/GB message delivery
/// We estimate based on destination type assuming ~10GB/month routed.
fn estimate_log_sink_cost(destination_type: &str, filter: &str) -> Option<f64> {
    // Base: assume ~10GB/month of logs routed through this sink
    let assumed_gb_per_month = 10.0;

    // Ingestion cost ($0.50/GB, but first 50GB free at project level)
    // We attribute a share per-sink; this is a rough estimate
    let ingestion_cost = assumed_gb_per_month * 0.50;

    let destination_cost = match destination_type {
        "bigquery" => assumed_gb_per_month * 0.05,       // streaming insert
        "cloud_storage" => assumed_gb_per_month * 0.02,   // GCS storage
        "pubsub" => assumed_gb_per_month * 0.04,          // Pub/Sub delivery
        "logging_bucket" => 0.0,                          // included in logging pricing
        _ => 0.0,
    };

    // Filtered sinks typically handle less data
    let filter_discount = if filter.is_empty() { 1.0 } else { 0.5 };

    Some((ingestion_cost + destination_cost) * filter_discount)
}
