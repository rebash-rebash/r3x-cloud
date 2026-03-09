use crate::cloud::provider::*;
use std::collections::HashMap;

/// Scan all Artifact Registry repositories in the project.
pub async fn scan_artifact_registry_repos(
    provider: &super::provider::GcpProvider,
    progress_tx: &tokio::sync::mpsc::Sender<ScanProgress>,
) -> anyhow::Result<Vec<CloudResource>> {
    let token = provider.get_access_token().await?;

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "ArtifactRegistryRepo".to_string(),
            found: 0,
            status: ScanStepStatus::Scanning,
        })
        .await;

    let url = format!(
        "https://artifactregistry.googleapis.com/v1/projects/{}/locations/-/repositories",
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

        if status.as_u16() == 403 || status.as_u16() == 404 {
            let _ = progress_tx
                .send(ScanProgress {
                    account_id: provider.project_id.clone(),
                    resource_type: "ArtifactRegistryRepo".to_string(),
                    found: 0,
                    status: ScanStepStatus::Completed,
                })
                .await;
            return Ok(vec![]);
        }

        let _ = progress_tx
            .send(ScanProgress {
                account_id: provider.project_id.clone(),
                resource_type: "ArtifactRegistryRepo".to_string(),
                found: 0,
                status: ScanStepStatus::Failed,
            })
            .await;

        return Err(anyhow::anyhow!(
            "Failed to list Artifact Registry repos: {} {}",
            status,
            body
        ));
    }

    let data: serde_json::Value = resp.json().await?;
    let mut resources = Vec::new();

    if let Some(repos) = data["repositories"].as_array() {
        for repo in repos {
            if let Some(resource) = parse_artifact_registry_repo(repo, &provider.project_id) {
                resources.push(resource);
            }
        }
    }

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "ArtifactRegistryRepo".to_string(),
            found: resources.len(),
            status: ScanStepStatus::Completed,
        })
        .await;

    Ok(resources)
}

fn parse_artifact_registry_repo(
    repo: &serde_json::Value,
    project_id: &str,
) -> Option<CloudResource> {
    let full_name = repo["name"].as_str()?;
    // Full name format: projects/{project}/locations/{location}/repositories/{repo}
    let name = full_name.rsplit('/').next().unwrap_or(full_name).to_string();

    let format = repo["format"].as_str().unwrap_or("unknown").to_string();
    let mode = repo["mode"].as_str().unwrap_or("STANDARD_REPOSITORY").to_string();
    let size_bytes = repo["sizeBytes"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| repo["sizeBytes"].as_f64())
        .unwrap_or(0.0);
    let created_at = repo["createTime"].as_str().map(String::from);
    let update_time = repo["updateTime"].as_str().unwrap_or("").to_string();

    // Extract region from the full name path
    let region = full_name
        .split('/')
        .nth(3) // locations/{location} -> the location value
        .unwrap_or("unknown")
        .to_string();

    let mut tags = HashMap::new();
    if let Some(labels) = repo["labels"].as_object() {
        for (k, v) in labels {
            if let Some(val) = v.as_str() {
                tags.insert(k.clone(), val.to_string());
            }
        }
    }

    let size_gb = size_bytes / (1024.0 * 1024.0 * 1024.0);

    let metadata = serde_json::json!({
        "format": format,
        "mode": mode,
        "size_bytes": size_bytes,
        "size_gb": size_gb,
        "update_time": update_time,
        "full_name": full_name,
    });

    let cost = estimate_artifact_registry_cost(size_gb);

    Some(CloudResource {
        id: name.clone(),
        name,
        resource_type: ResourceType::ArtifactRegistryRepo,
        provider: ProviderKind::Gcp,
        region,
        account_id: project_id.to_string(),
        status: "ACTIVE".to_string(),
        created_at,
        last_used: None,
        tags,
        metadata,
        monthly_cost: cost,
    })
}

/// Rough cost estimate for Artifact Registry ($0.10/GB/month storage).
fn estimate_artifact_registry_cost(size_gb: f64) -> Option<f64> {
    // $0.10/GB/month, minimum $0.01 if any data exists
    if size_gb > 0.0 {
        Some((size_gb * 0.10).max(0.01))
    } else {
        Some(0.0)
    }
}
