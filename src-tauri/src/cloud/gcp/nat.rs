use crate::cloud::provider::*;
use std::collections::HashMap;

/// Scan all Cloud NAT gateways in the project (nested inside routers).
pub async fn scan_nat_gateways(
    provider: &super::provider::GcpProvider,
    progress_tx: &tokio::sync::mpsc::Sender<ScanProgress>,
) -> anyhow::Result<Vec<CloudResource>> {
    let token = provider.get_access_token().await?;

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "NatGateway".to_string(),
            found: 0,
            status: ScanStepStatus::Scanning,
        })
        .await;

    let url = format!(
        "https://compute.googleapis.com/compute/v1/projects/{}/aggregated/routers",
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
                    resource_type: "NatGateway".to_string(),
                    found: 0,
                    status: ScanStepStatus::Completed,
                })
                .await;
            return Ok(vec![]);
        }

        let _ = progress_tx
            .send(ScanProgress {
                account_id: provider.project_id.clone(),
                resource_type: "NatGateway".to_string(),
                found: 0,
                status: ScanStepStatus::Failed,
            })
            .await;

        return Err(anyhow::anyhow!("Failed to list routers: {} {}", status, body));
    }

    let data: serde_json::Value = resp.json().await?;
    let mut resources = Vec::new();

    if let Some(items) = data["items"].as_object() {
        for (scope_key, scope_val) in items {
            let region = scope_key
                .strip_prefix("regions/")
                .unwrap_or("unknown")
                .to_string();

            if let Some(routers) = scope_val["routers"].as_array() {
                for router in routers {
                    if let Some(nats) = router["nats"].as_array() {
                        let router_name = router["name"]
                            .as_str()
                            .unwrap_or("unknown")
                            .to_string();

                        for nat in nats {
                            if let Some(resource) = parse_nat_gateway(
                                nat,
                                &router_name,
                                &region,
                                &provider.project_id,
                            ) {
                                resources.push(resource);
                            }
                        }
                    }
                }
            }
        }
    }

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "NatGateway".to_string(),
            found: resources.len(),
            status: ScanStepStatus::Completed,
        })
        .await;

    Ok(resources)
}

fn parse_nat_gateway(
    nat: &serde_json::Value,
    router_name: &str,
    region: &str,
    project_id: &str,
) -> Option<CloudResource> {
    let name = nat["name"].as_str()?.to_string();

    let nat_ip_allocate_option = nat["natIpAllocateOption"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();
    let source_subnetwork_ip_ranges = nat["sourceSubnetworkIpRangesToNat"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    let metadata = serde_json::json!({
        "router_name": router_name,
        "nat_ip_allocate_option": nat_ip_allocate_option,
        "source_subnetwork_ip_ranges_to_nat": source_subnetwork_ip_ranges,
    });

    let cost = estimate_nat_cost();

    Some(CloudResource {
        id: format!("{}/{}", router_name, name),
        name,
        resource_type: ResourceType::NatGateway,
        provider: ProviderKind::Gcp,
        region: region.to_string(),
        account_id: project_id.to_string(),
        status: "ACTIVE".to_string(),
        created_at: None,
        last_used: None,
        tags: HashMap::new(),
        metadata,
        monthly_cost: cost,
    })
}

/// Rough cost estimate for Cloud NAT (~$0.044/hour per gateway = ~$31.68/month,
/// plus $0.045/GB processed — data charges not included here).
fn estimate_nat_cost() -> Option<f64> {
    // $0.044/hour * 720 hours/month ≈ $31.68/month
    Some(31.68)
}
