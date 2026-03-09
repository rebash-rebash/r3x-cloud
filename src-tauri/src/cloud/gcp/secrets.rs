use crate::cloud::provider::*;
use std::collections::HashMap;

/// Scan all Secret Manager secrets in the project.
pub async fn scan_secrets(
    provider: &super::provider::GcpProvider,
    progress_tx: &tokio::sync::mpsc::Sender<ScanProgress>,
) -> anyhow::Result<Vec<CloudResource>> {
    let token = provider.get_access_token().await?;

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "SecretManagerSecret".to_string(),
            found: 0,
            status: ScanStepStatus::Scanning,
        })
        .await;

    let url = format!(
        "https://secretmanager.googleapis.com/v1/projects/{}/secrets",
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

        // Secret Manager API might not be enabled
        if status.as_u16() == 403 || status.as_u16() == 404 {
            let _ = progress_tx
                .send(ScanProgress {
                    account_id: provider.project_id.clone(),
                    resource_type: "SecretManagerSecret".to_string(),
                    found: 0,
                    status: ScanStepStatus::Completed,
                })
                .await;
            return Ok(vec![]);
        }

        let _ = progress_tx
            .send(ScanProgress {
                account_id: provider.project_id.clone(),
                resource_type: "SecretManagerSecret".to_string(),
                found: 0,
                status: ScanStepStatus::Failed,
            })
            .await;

        return Err(anyhow::anyhow!("Failed to list secrets: {} {}", status, body));
    }

    let data: serde_json::Value = resp.json().await?;
    let mut resources = Vec::new();

    if let Some(secrets) = data["secrets"].as_array() {
        for secret in secrets {
            if let Some(resource) = parse_secret(secret, &provider.project_id, provider, &token).await {
                resources.push(resource);
            }
        }
    }

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "SecretManagerSecret".to_string(),
            found: resources.len(),
            status: ScanStepStatus::Completed,
        })
        .await;

    Ok(resources)
}

async fn parse_secret(
    secret: &serde_json::Value,
    project_id: &str,
    provider: &super::provider::GcpProvider,
    token: &str,
) -> Option<CloudResource> {
    // name is full path like "projects/123456/secrets/my-secret"
    let full_name = secret["name"].as_str()?;
    let short_name = full_name.rsplit('/').next().unwrap_or(full_name).to_string();

    let created_at = secret["createTime"].as_str().map(String::from);

    let replication = &secret["replication"];
    let replication_type = if replication["automatic"].is_object() {
        "automatic"
    } else if replication["userManaged"].is_object() {
        "user_managed"
    } else {
        "unknown"
    };

    let mut tags = HashMap::new();
    if let Some(labels) = secret["labels"].as_object() {
        for (k, v) in labels {
            if let Some(val) = v.as_str() {
                tags.insert(k.clone(), val.to_string());
            }
        }
    }

    // Check latest version status (don't read the secret value!)
    let (status, version_count) = check_secret_versions(provider, token, full_name).await;

    let metadata = serde_json::json!({
        "full_name": full_name,
        "replication_type": replication_type,
        "replication": replication,
        "version_count": version_count,
    });

    // Cost: $0.03/active version/month + $0.06/10k access operations
    let cost = estimate_secret_cost(version_count);

    Some(CloudResource {
        id: short_name.clone(),
        name: short_name,
        resource_type: ResourceType::SecretManagerSecret,
        provider: ProviderKind::Gcp,
        region: "global".to_string(),
        account_id: project_id.to_string(),
        status,
        created_at,
        last_used: None,
        tags,
        metadata,
        monthly_cost: cost,
    })
}

/// Check if a secret has enabled versions without reading the actual value.
async fn check_secret_versions(
    provider: &super::provider::GcpProvider,
    token: &str,
    secret_full_name: &str,
) -> (String, u64) {
    let url = format!(
        "https://secretmanager.googleapis.com/v1/{}/versions",
        secret_full_name
    );

    let resp = provider
        .client
        .get(&url)
        .bearer_auth(token)
        .query(&[("pageSize", "100")])
        .send()
        .await;

    let resp = match resp {
        Ok(r) => r,
        Err(_) => return ("UNKNOWN".to_string(), 0),
    };

    if !resp.status().is_success() {
        return ("UNKNOWN".to_string(), 0);
    }

    let data: serde_json::Value = match resp.json().await {
        Ok(d) => d,
        Err(_) => return ("UNKNOWN".to_string(), 0),
    };

    let versions = data["versions"].as_array();
    let total = data["totalSize"]
        .as_u64()
        .or_else(|| data["totalSize"].as_str().and_then(|s| s.parse().ok()))
        .unwrap_or_else(|| versions.map(|v| v.len() as u64).unwrap_or(0));

    let has_enabled = versions
        .map(|vs| {
            vs.iter().any(|v| {
                v["state"].as_str() == Some("ENABLED")
            })
        })
        .unwrap_or(false);

    let status = if has_enabled {
        "ACTIVE".to_string()
    } else {
        "DISABLED".to_string()
    };

    (status, total)
}

/// Rough cost estimate for Secret Manager secrets.
/// $0.03/active version/month + $0.06/10,000 access operations (assume minimal access).
fn estimate_secret_cost(version_count: u64) -> Option<f64> {
    let version_cost = version_count as f64 * 0.03;
    // Assume ~1,000 access operations/month as baseline
    let access_cost = 0.006;
    Some(version_cost + access_cost)
}
