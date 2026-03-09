use crate::cloud::provider::*;
use std::collections::HashMap;

/// Scan all VPN tunnels in the project.
pub async fn scan_vpn_tunnels(
    provider: &super::provider::GcpProvider,
    progress_tx: &tokio::sync::mpsc::Sender<ScanProgress>,
) -> anyhow::Result<Vec<CloudResource>> {
    let token = provider.get_access_token().await?;

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "VpnTunnel".to_string(),
            found: 0,
            status: ScanStepStatus::Scanning,
        })
        .await;

    let url = format!(
        "https://compute.googleapis.com/compute/v1/projects/{}/aggregated/vpnTunnels",
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
                    resource_type: "VpnTunnel".to_string(),
                    found: 0,
                    status: ScanStepStatus::Completed,
                })
                .await;
            return Ok(vec![]);
        }

        let _ = progress_tx
            .send(ScanProgress {
                account_id: provider.project_id.clone(),
                resource_type: "VpnTunnel".to_string(),
                found: 0,
                status: ScanStepStatus::Failed,
            })
            .await;

        return Err(anyhow::anyhow!("Failed to list VPN tunnels: {} {}", status, body));
    }

    let data: serde_json::Value = resp.json().await?;
    let mut resources = Vec::new();

    if let Some(items) = data["items"].as_object() {
        for (scope_key, scope_val) in items {
            let region = scope_key
                .strip_prefix("regions/")
                .unwrap_or("unknown")
                .to_string();

            if let Some(tunnels) = scope_val["vpnTunnels"].as_array() {
                for tunnel in tunnels {
                    if let Some(resource) = parse_vpn_tunnel(tunnel, &region, &provider.project_id)
                    {
                        resources.push(resource);
                    }
                }
            }
        }
    }

    let _ = progress_tx
        .send(ScanProgress {
            account_id: provider.project_id.clone(),
            resource_type: "VpnTunnel".to_string(),
            found: resources.len(),
            status: ScanStepStatus::Completed,
        })
        .await;

    Ok(resources)
}

fn parse_vpn_tunnel(
    tunnel: &serde_json::Value,
    region: &str,
    project_id: &str,
) -> Option<CloudResource> {
    let name = tunnel["name"].as_str()?.to_string();
    let status = tunnel["status"].as_str().unwrap_or("UNKNOWN").to_string();
    let peer_ip = tunnel["peerIp"].as_str().unwrap_or("unknown").to_string();
    let ike_version = tunnel["ikeVersion"].as_i64().unwrap_or(0);
    let detailed_status = tunnel["detailedStatus"].as_str().unwrap_or("").to_string();
    let created_at = tunnel["creationTimestamp"].as_str().map(String::from);

    let gateway = tunnel["vpnGateway"]
        .as_str()
        .or_else(|| tunnel["targetVpnGateway"].as_str())
        .unwrap_or("unknown")
        .to_string();

    let mut tags = HashMap::new();
    if let Some(labels) = tunnel["labels"].as_object() {
        for (k, v) in labels {
            if let Some(val) = v.as_str() {
                tags.insert(k.clone(), val.to_string());
            }
        }
    }

    let metadata = serde_json::json!({
        "peer_ip": peer_ip,
        "ike_version": ike_version,
        "detailed_status": detailed_status,
        "gateway": gateway,
        "self_link": tunnel["selfLink"],
    });

    let cost = estimate_vpn_cost();

    Some(CloudResource {
        id: name.clone(),
        name,
        resource_type: ResourceType::VpnTunnel,
        provider: ProviderKind::Gcp,
        region: region.to_string(),
        account_id: project_id.to_string(),
        status,
        created_at,
        last_used: None,
        tags,
        metadata,
        monthly_cost: cost,
    })
}

/// Rough cost estimate for VPN tunnels (~$0.075/hour = ~$54/month per tunnel).
fn estimate_vpn_cost() -> Option<f64> {
    // $0.075/hour * 720 hours/month = $54/month
    Some(54.0)
}
