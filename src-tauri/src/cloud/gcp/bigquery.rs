use crate::cloud::provider::*;
use std::collections::HashMap;

/// Scan all BigQuery datasets in the project.
pub async fn scan_bigquery_datasets(
    provider: &super::provider::GcpProvider,
    progress_tx: &tokio::sync::mpsc::Sender<ScanProgress>,
) -> anyhow::Result<Vec<CloudResource>> {
    let token = provider.get_access_token().await?;

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "BigQueryDataset".to_string(),
            found: 0,
            status: ScanStepStatus::Scanning,
        })
        .await;

    let url = format!(
        "https://bigquery.googleapis.com/bigquery/v2/projects/{}/datasets",
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

        // BigQuery API might not be enabled
        if status.as_u16() == 403 || status.as_u16() == 404 {
            let _ = progress_tx
                .send(ScanProgress {
                    account_id: provider.project_id.clone(),
                    resource_type: "BigQueryDataset".to_string(),
                    found: 0,
                    status: ScanStepStatus::Completed,
                })
                .await;
            return Ok(vec![]);
        }

        let _ = progress_tx
            .send(ScanProgress {
                account_id: provider.project_id.clone(),
                resource_type: "BigQueryDataset".to_string(),
                found: 0,
                status: ScanStepStatus::Failed,
            })
            .await;

        return Err(anyhow::anyhow!("Failed to list BigQuery datasets: {} {}", status, body));
    }

    let data: serde_json::Value = resp.json().await?;
    let mut resources = Vec::new();

    if let Some(datasets) = data["datasets"].as_array() {
        for dataset_ref in datasets {
            let dataset_id = match dataset_ref["datasetReference"]["datasetId"].as_str() {
                Some(id) => id.to_string(),
                None => continue,
            };

            if let Some(resource) = fetch_dataset_details(
                provider,
                &token,
                &dataset_id,
            )
            .await
            {
                resources.push(resource);
            }
        }
    }

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "BigQueryDataset".to_string(),
            found: resources.len(),
            status: ScanStepStatus::Completed,
        })
        .await;

    Ok(resources)
}

/// Fetch dataset details and its tables, then build the CloudResource.
async fn fetch_dataset_details(
    provider: &super::provider::GcpProvider,
    token: &str,
    dataset_id: &str,
) -> Option<CloudResource> {
    // Get dataset details
    let detail_url = format!(
        "https://bigquery.googleapis.com/bigquery/v2/projects/{}/datasets/{}",
        provider.project_id, dataset_id
    );

    let detail_resp = provider
        .client
        .get(&detail_url)
        .bearer_auth(token)
        .send()
        .await
        .ok()?;

    if !detail_resp.status().is_success() {
        return None;
    }

    let dataset: serde_json::Value = detail_resp.json().await.ok()?;

    let location = dataset["location"].as_str().unwrap_or("unknown").to_string();
    let default_table_expiration_ms = dataset["defaultTableExpirationMs"]
        .as_str()
        .or_else(|| dataset["defaultTableExpirationMs"].as_f64().map(|_| ""))
        .map(String::from);
    let creation_time = dataset["creationTime"].as_str().map(String::from);
    let last_modified_time = dataset["lastModifiedTime"].as_str().map(String::from);

    let mut tags = HashMap::new();
    if let Some(labels) = dataset["labels"].as_object() {
        for (k, v) in labels {
            if let Some(val) = v.as_str() {
                tags.insert(k.clone(), val.to_string());
            }
        }
    }

    // List tables for this dataset
    let tables_url = format!(
        "https://bigquery.googleapis.com/bigquery/v2/projects/{}/datasets/{}/tables?maxResults=100",
        provider.project_id, dataset_id
    );

    let mut table_count: u64 = 0;
    let mut total_bytes: u64 = 0;

    if let Ok(tables_resp) = provider
        .client
        .get(&tables_url)
        .bearer_auth(token)
        .send()
        .await
    {
        if tables_resp.status().is_success() {
            if let Ok(tables_data) = tables_resp.json::<serde_json::Value>().await {
                if let Some(tables) = tables_data["tables"].as_array() {
                    table_count = tables.len() as u64;
                    for table in tables {
                        let num_bytes = table["numBytes"]
                            .as_str()
                            .and_then(|s| s.parse::<u64>().ok())
                            .or_else(|| table["numBytes"].as_u64())
                            .unwrap_or(0);
                        total_bytes += num_bytes;
                    }
                }
            }
        }
    }

    let total_size_gb = total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

    let status = if table_count > 0 {
        "ACTIVE".to_string()
    } else {
        "EMPTY".to_string()
    };

    let metadata = serde_json::json!({
        "dataset_id": dataset_id,
        "location": location,
        "default_table_expiration_ms": default_table_expiration_ms,
        "creation_time": creation_time,
        "last_modified_time": last_modified_time,
        "access": dataset["access"],
        "table_count": table_count,
        "total_bytes": total_bytes,
        "total_size_gb": total_size_gb,
        "self_link": dataset["selfLink"],
    });

    let cost = estimate_bigquery_cost(total_size_gb);

    Some(CloudResource {
        id: dataset_id.to_string(),
        name: dataset_id.to_string(),
        resource_type: ResourceType::BigQueryDataset,
        provider: ProviderKind::Gcp,
        region: location,
        account_id: provider.project_id.to_string(),
        status,
        created_at: creation_time,
        last_used: last_modified_time,
        tags,
        metadata,
        monthly_cost: cost,
    })
}

/// Rough cost estimate for BigQuery storage.
/// Active storage: $0.02/GB/month, long-term (>90 days): $0.01/GB/month.
/// Since we can't determine age of data from listing, we use active storage rate.
fn estimate_bigquery_cost(total_size_gb: f64) -> Option<f64> {
    if total_size_gb <= 0.0 {
        return Some(0.0);
    }

    // Use active storage rate ($0.02/GB/month) as default
    let cost = total_size_gb * 0.02;

    Some(cost)
}
