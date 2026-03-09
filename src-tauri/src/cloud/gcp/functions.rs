use crate::cloud::provider::*;
use std::collections::HashMap;

/// Scan all Cloud Functions (v2) in the project across all regions.
pub async fn scan_functions(
    provider: &super::provider::GcpProvider,
    progress_tx: &tokio::sync::mpsc::Sender<ScanProgress>,
) -> anyhow::Result<Vec<CloudResource>> {
    let token = provider.get_access_token().await?;

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "ServerlessFunction".to_string(),
            found: 0,
            status: ScanStepStatus::Scanning,
        })
        .await;

    // Use wildcard location "-" to list functions across all regions
    let url = format!(
        "https://cloudfunctions.googleapis.com/v2/projects/{}/locations/-/functions",
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

        // Cloud Functions API might not be enabled — treat as 0 resources
        if status.as_u16() == 403 || status.as_u16() == 404 {
            let _ = progress_tx
                .send(ScanProgress {
                    account_id: provider.project_id.clone(),
                    resource_type: "ServerlessFunction".to_string(),
                    found: 0,
                    status: ScanStepStatus::Completed,
                })
                .await;
            return Ok(vec![]);
        }

        let _ = progress_tx
            .send(ScanProgress {
                account_id: provider.project_id.clone(),
                resource_type: "ServerlessFunction".to_string(),
                found: 0,
                status: ScanStepStatus::Failed,
            })
            .await;

        return Err(anyhow::anyhow!("Failed to list functions: {} {}", status, body));
    }

    let data: serde_json::Value = resp.json().await?;
    let mut resources = Vec::new();

    if let Some(functions) = data["functions"].as_array() {
        for func in functions {
            if let Some(resource) = parse_function(func, &provider.project_id) {
                resources.push(resource);
            }
        }
    }

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "ServerlessFunction".to_string(),
            found: resources.len(),
            status: ScanStepStatus::Completed,
        })
        .await;

    Ok(resources)
}

fn parse_function(func: &serde_json::Value, project_id: &str) -> Option<CloudResource> {
    // v2 name format: projects/{project}/locations/{location}/functions/{name}
    let full_name = func["name"].as_str()?;
    let parts: Vec<&str> = full_name.split('/').collect();
    let name = parts.last()?.to_string();
    let location = parts.get(3).unwrap_or(&"unknown").to_string();

    let state = func["state"].as_str().unwrap_or("UNKNOWN").to_string();
    let runtime = func["buildConfig"]["runtime"].as_str().unwrap_or("unknown");
    let entry_point = func["buildConfig"]["entryPoint"].as_str().unwrap_or("");
    let environment = func["environment"].as_str().unwrap_or("GEN_2");
    let update_time = func["updateTime"].as_str().map(String::from);
    let create_time = func["createTime"].as_str().map(String::from);

    // Service config details
    let memory = func["serviceConfig"]["availableMemory"].as_str().unwrap_or("256Mi");
    let timeout = func["serviceConfig"]["timeoutSeconds"].as_str()
        .or_else(|| func["serviceConfig"]["timeoutSeconds"].as_i64().map(|_| ""))
        .unwrap_or("60");
    let max_instances = func["serviceConfig"]["maxInstanceCount"].as_i64().unwrap_or(0);
    let min_instances = func["serviceConfig"]["minInstanceCount"].as_i64().unwrap_or(0);

    let mut tags = HashMap::new();
    if let Some(labels) = func["labels"].as_object() {
        for (k, v) in labels {
            if let Some(val) = v.as_str() {
                tags.insert(k.clone(), val.to_string());
            }
        }
    }

    let metadata = serde_json::json!({
        "runtime": runtime,
        "entry_point": entry_point,
        "environment": environment,
        "memory": memory,
        "timeout_seconds": timeout,
        "max_instances": max_instances,
        "min_instances": min_instances,
        "url": func["serviceConfig"]["uri"],
        "trigger": func["eventTrigger"],
    });

    // Cost: Cloud Functions pricing is per-invocation + compute time
    // Min instances have a baseline cost
    let cost = if min_instances > 0 {
        Some(min_instances as f64 * 5.40) // ~$5.40/month per idle min instance (256MB)
    } else {
        Some(0.0) // Pay per use — cost depends on invocations
    };

    Some(CloudResource {
        id: full_name.to_string(),
        name,
        resource_type: ResourceType::ServerlessFunction,
        provider: ProviderKind::Gcp,
        region: location,
        account_id: project_id.to_string(),
        status: state,
        created_at: create_time,
        last_used: update_time,
        tags,
        metadata,
        monthly_cost: cost,
    })
}
