use crate::cloud::provider::*;
use std::collections::HashMap;

/// Scan all App Engine service versions in the project.
pub async fn scan_appengine_versions(
    provider: &super::provider::GcpProvider,
    progress_tx: &tokio::sync::mpsc::Sender<ScanProgress>,
) -> anyhow::Result<Vec<CloudResource>> {
    let token = provider.get_access_token().await?;

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "AppEngineVersion".to_string(),
            found: 0,
            status: ScanStepStatus::Scanning,
        })
        .await;

    // First, list all services
    let services_url = format!(
        "https://appengine.googleapis.com/v1/apps/{}/services",
        provider.project_id
    );

    let resp = provider
        .client
        .get(&services_url)
        .bearer_auth(&token)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();

        // App Engine API might not be enabled or no app exists
        if status.as_u16() == 403 || status.as_u16() == 404 {
            let _ = progress_tx
                .send(ScanProgress {
                    account_id: provider.project_id.clone(),
                    resource_type: "AppEngineVersion".to_string(),
                    found: 0,
                    status: ScanStepStatus::Completed,
                })
                .await;
            return Ok(vec![]);
        }

        let _ = progress_tx
            .send(ScanProgress {
                account_id: provider.project_id.clone(),
                resource_type: "AppEngineVersion".to_string(),
                found: 0,
                status: ScanStepStatus::Failed,
            })
            .await;

        return Err(anyhow::anyhow!("Failed to list App Engine services: {} {}", status, body));
    }

    let services_data: serde_json::Value = resp.json().await?;
    let mut resources = Vec::new();

    if let Some(services) = services_data["services"].as_array() {
        for service in services {
            let service_id = match service["id"].as_str() {
                Some(id) => id,
                None => continue,
            };

            // For each service, list its versions
            let versions_url = format!(
                "https://appengine.googleapis.com/v1/apps/{}/services/{}/versions",
                provider.project_id, service_id
            );

            let versions_resp = provider
                .client
                .get(&versions_url)
                .bearer_auth(&token)
                .send()
                .await?;

            if !versions_resp.status().is_success() {
                let status = versions_resp.status();

                // Skip services we can't access
                if status.as_u16() == 403 || status.as_u16() == 404 {
                    continue;
                }

                let body = versions_resp.text().await.unwrap_or_default();
                return Err(anyhow::anyhow!(
                    "Failed to list App Engine versions for service {}: {} {}",
                    service_id, status, body
                ));
            }

            let versions_data: serde_json::Value = versions_resp.json().await?;

            if let Some(versions) = versions_data["versions"].as_array() {
                for version in versions {
                    if let Some(resource) = parse_appengine_version(
                        version,
                        service_id,
                        &provider.project_id,
                    ) {
                        resources.push(resource);
                    }
                }
            }
        }
    }

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "AppEngineVersion".to_string(),
            found: resources.len(),
            status: ScanStepStatus::Completed,
        })
        .await;

    Ok(resources)
}

fn parse_appengine_version(
    version: &serde_json::Value,
    service_id: &str,
    project_id: &str,
) -> Option<CloudResource> {
    let id = version["id"].as_str()?.to_string();
    let runtime = version["runtime"].as_str().unwrap_or("unknown");
    let serving_status = version["servingStatus"].as_str().unwrap_or("UNKNOWN");
    let created_at = version["createTime"].as_str().map(String::from);
    let version_url = version["versionUrl"].as_str().unwrap_or("unknown");
    let instance_class = version["instanceClass"].as_str().unwrap_or("F1");

    let name = format!("{}/{}", service_id, id);

    let metadata = serde_json::json!({
        "service": service_id,
        "version_id": id,
        "runtime": runtime,
        "serving_status": serving_status,
        "version_url": version_url,
        "instance_class": instance_class,
    });

    let status = match serving_status {
        "SERVING" => "SERVING".to_string(),
        "STOPPED" => "STOPPED".to_string(),
        other => other.to_string(),
    };

    // Only estimate cost for running versions
    let cost = if serving_status == "SERVING" {
        estimate_appengine_cost(instance_class)
    } else {
        Some(0.0)
    };

    Some(CloudResource {
        id: name.clone(),
        name,
        resource_type: ResourceType::AppEngineVersion,
        provider: ProviderKind::Gcp,
        region: project_id.to_string(), // App Engine is per-project, region set at app level
        account_id: project_id.to_string(),
        status,
        created_at,
        last_used: None,
        tags: HashMap::new(),
        metadata,
        monthly_cost: cost,
    })
}

/// Rough cost estimate for App Engine instances (per-instance, on-demand).
/// F1: ~$0.05/hour = ~$36/month
/// F2: ~$0.10/hour = ~$72/month
/// F4: ~$0.20/hour = ~$144/month
/// B1: ~$0.05/hour = ~$36/month
/// B2: ~$0.10/hour = ~$72/month
/// B4: ~$0.20/hour = ~$144/month
/// B8: ~$0.40/hour = ~$288/month
fn estimate_appengine_cost(instance_class: &str) -> Option<f64> {
    let monthly = match instance_class {
        "F1" | "B1" => 36.0,
        "F2" | "B2" => 72.0,
        "F4" | "B4" => 144.0,
        "F4_1G" | "B4_1G" => 144.0,
        "B8" => 288.0,
        _ => 36.0, // default to smallest
    };

    Some(monthly)
}
