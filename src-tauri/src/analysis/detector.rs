use crate::cloud::provider::CloudResource;
use crate::analysis::rules::{Severity, all_gcp_rules};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub resource_id: String,
    pub resource_name: String,
    pub resource_type: String,
    pub rule_id: String,
    pub rule_name: String,
    pub severity: Severity,
    pub description: String,
    pub recommendation: String,
    pub estimated_monthly_savings: f64,
    pub region: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisSummary {
    pub total_findings: usize,
    pub total_monthly_savings: f64,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    pub findings: Vec<Finding>,
}

/// Analyze scanned resources and detect unused/wasteful resources.
/// `rule_overrides` maps rule_id -> enabled, overriding the default enabled state.
pub fn analyze_resources(resources: &[CloudResource], rule_overrides: &HashMap<String, bool>) -> AnalysisSummary {
    let mut rules = all_gcp_rules();

    // Apply user overrides from saved configs
    for rule in &mut rules {
        if let Some(&enabled) = rule_overrides.get(&rule.id) {
            rule.enabled = enabled;
        }
    }

    let mut findings = Vec::new();

    for resource in resources {
        for rule in &rules {
            if !rule.enabled {
                continue;
            }

            // Wildcard rules apply to all resource types
            if rule.resource_type != "*" {
                let type_str = serde_json::to_string(&resource.resource_type)
                    .unwrap_or_default()
                    .trim_matches('"')
                    .to_string();
                if type_str != rule.resource_type {
                    // Skip if resource type doesn't match (optimization for non-wildcard rules)
                    // But check_rule already handles this, so just let it pass
                }
            }

            if let Some(finding) = check_rule(resource, rule) {
                findings.push(finding);
            }
        }
    }

    // Sort by savings (highest first)
    findings.sort_by(|a, b| b.estimated_monthly_savings.partial_cmp(&a.estimated_monthly_savings).unwrap_or(std::cmp::Ordering::Equal));

    let total_savings = findings.iter().map(|f| f.estimated_monthly_savings).sum();
    let critical = findings.iter().filter(|f| matches!(f.severity, Severity::Critical)).count();
    let high = findings.iter().filter(|f| matches!(f.severity, Severity::High)).count();
    let medium = findings.iter().filter(|f| matches!(f.severity, Severity::Medium)).count();
    let low = findings.iter().filter(|f| matches!(f.severity, Severity::Low)).count();

    AnalysisSummary {
        total_findings: findings.len(),
        total_monthly_savings: total_savings,
        critical_count: critical,
        high_count: high,
        medium_count: medium,
        low_count: low,
        findings,
    }
}

fn check_rule(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    match rule.id.as_str() {
        "gcp-vm-stopped" => check_vm_stopped(resource, rule),
        "gcp-disk-unattached" => check_disk_unattached(resource, rule),
        "gcp-disk-large-unused" => check_disk_large_unused(resource, rule),
        "gcp-snapshot-old" => check_snapshot_old(resource, rule),
        "gcp-ip-unused" => check_ip_unused(resource, rule),
        "gcp-lb-no-target" => check_lb_no_target(resource, rule),
        "gcp-fw-disabled" => check_fw_disabled(resource, rule),
        "gcp-image-old" => check_image_old(resource, rule),
        "gcp-sql-stopped" => check_sql_stopped(resource, rule),
        "gcp-function-idle" => check_function_idle(resource, rule),
        "gcp-run-idle" => check_run_idle(resource, rule),
        "gcp-resource-untagged" => check_untagged(resource, rule),
        "gcp-gke-degraded" => check_gke_degraded(resource, rule),
        "gcp-gke-overprovisioned" => check_gke_overprovisioned(resource, rule),
        "gcp-bq-empty" => check_bq_empty(resource, rule),
        "gcp-bq-large" => check_bq_large(resource, rule),
        "gcp-pubsub-detached" => check_pubsub_detached(resource, rule),
        "gcp-spanner-idle" => check_spanner_idle(resource, rule),
        "gcp-memorystore-idle" => check_memorystore_idle(resource, rule),
        "gcp-appengine-stopped" => check_appengine_stopped(resource, rule),
        "gcp-nat-unused" => check_nat_unused(resource, rule),
        "gcp-vpn-down" => check_vpn_down(resource, rule),
        "gcp-artifact-large" => check_artifact_large(resource, rule),
        "gcp-dataproc-idle" => check_dataproc_idle(resource, rule),
        "gcp-secret-disabled" => check_secret_disabled(resource, rule),
        "gcp-logsink-disabled" => check_logsink_disabled(resource, rule),
        _ => None,
    }
}

fn make_finding(
    resource: &CloudResource,
    rule: &crate::analysis::rules::DetectionRule,
    description: String,
    recommendation: String,
    savings: f64,
) -> Finding {
    Finding {
        id: format!("{}-{}", rule.id, resource.id),
        resource_id: resource.id.clone(),
        resource_name: resource.name.clone(),
        resource_type: serde_json::to_string(&resource.resource_type).unwrap_or_default().trim_matches('"').to_string(),
        rule_id: rule.id.clone(),
        rule_name: rule.name.clone(),
        severity: rule.severity.clone(),
        description,
        recommendation,
        estimated_monthly_savings: savings,
        region: resource.region.clone(),
    }
}

fn check_vm_stopped(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    if !matches!(&resource.resource_type, crate::cloud::provider::ResourceType::VirtualMachine) {
        return None;
    }
    if resource.status.to_uppercase() == "TERMINATED" {
        let savings = resource.monthly_cost.unwrap_or(0.0);
        let machine_type = resource.metadata.get("machine_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        Some(make_finding(
            resource,
            rule,
            format!("VM '{}' ({}) is stopped/terminated in {}", resource.name, machine_type, resource.region),
            format!("Delete or restart this VM. Stopped VMs still incur costs for attached disks and reserved IPs."),
            savings,
        ))
    } else {
        None
    }
}

fn check_disk_unattached(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    if !matches!(&resource.resource_type, crate::cloud::provider::ResourceType::Disk) {
        return None;
    }
    if resource.status.to_uppercase() == "UNATTACHED" {
        let savings = resource.monthly_cost.unwrap_or(0.0);
        let size_gb = resource.metadata.get("size_gb").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let disk_type = resource.metadata.get("disk_type").and_then(|v| v.as_str()).unwrap_or("unknown");
        Some(make_finding(
            resource,
            rule,
            format!("Disk '{}' ({} GB, {}) is not attached to any instance", resource.name, size_gb, disk_type),
            "Snapshot this disk for backup, then delete it to save costs.".into(),
            savings,
        ))
    } else {
        None
    }
}

fn check_disk_large_unused(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    if !matches!(&resource.resource_type, crate::cloud::provider::ResourceType::Disk) {
        return None;
    }
    if resource.status.to_uppercase() != "UNATTACHED" {
        return None;
    }
    let size_gb = resource.metadata.get("size_gb").and_then(|v| v.as_f64()).unwrap_or(0.0);
    if size_gb >= 100.0 {
        let savings = resource.monthly_cost.unwrap_or(0.0);
        Some(make_finding(
            resource,
            rule,
            format!("Large unattached disk '{}' ({} GB) is wasting significant storage costs", resource.name, size_gb),
            "This disk is large and unattached. Delete it immediately or snapshot and delete.".into(),
            savings,
        ))
    } else {
        None
    }
}

fn check_snapshot_old(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    if !matches!(&resource.resource_type, crate::cloud::provider::ResourceType::Snapshot) {
        return None;
    }
    let created = resource.created_at.as_deref()?;
    let created_date = chrono::DateTime::parse_from_rfc3339(created).ok()?;
    let age_days = (chrono::Utc::now() - created_date.with_timezone(&chrono::Utc)).num_days();

    if age_days > 90 {
        let savings = resource.monthly_cost.unwrap_or(0.0);
        Some(make_finding(
            resource,
            rule,
            format!("Snapshot '{}' is {} days old", resource.name, age_days),
            "Review if this snapshot is still needed. Old snapshots accumulate storage costs.".into(),
            savings,
        ))
    } else {
        None
    }
}

fn check_ip_unused(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    if !matches!(&resource.resource_type, crate::cloud::provider::ResourceType::ElasticIp) {
        return None;
    }
    if resource.status.to_uppercase() == "RESERVED" {
        Some(make_finding(
            resource,
            rule,
            format!("Static IP '{}' ({}) is reserved but not in use",
                resource.name,
                resource.metadata.get("address").and_then(|v| v.as_str()).unwrap_or("unknown")),
            "Release this static IP or assign it to a resource. Unused IPs cost $7.30/month.".into(),
            7.30,
        ))
    } else {
        None
    }
}

fn check_lb_no_target(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    if !matches!(&resource.resource_type, crate::cloud::provider::ResourceType::LoadBalancer) {
        return None;
    }
    if resource.status.to_uppercase() == "NO_TARGET" {
        let savings = resource.monthly_cost.unwrap_or(18.26);
        Some(make_finding(
            resource,
            rule,
            format!("Forwarding rule '{}' has no backend target", resource.name),
            "This load balancer has no targets. Delete it if no longer needed.".into(),
            savings,
        ))
    } else {
        None
    }
}

fn check_fw_disabled(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    if !matches!(&resource.resource_type, crate::cloud::provider::ResourceType::SecurityGroup) {
        return None;
    }
    if resource.status.to_uppercase() == "DISABLED" {
        Some(make_finding(
            resource,
            rule,
            format!("Firewall rule '{}' is disabled", resource.name),
            "Review if this firewall rule is still needed. Disabled rules add clutter.".into(),
            0.0,
        ))
    } else {
        None
    }
}

fn check_image_old(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    if !matches!(&resource.resource_type, crate::cloud::provider::ResourceType::MachineImage) {
        return None;
    }
    let created = resource.created_at.as_deref()?;
    let created_date = chrono::DateTime::parse_from_rfc3339(created).ok()?;
    let age_days = (chrono::Utc::now() - created_date.with_timezone(&chrono::Utc)).num_days();

    if age_days > 180 {
        let savings = resource.monthly_cost.unwrap_or(0.0);
        Some(make_finding(
            resource,
            rule,
            format!("Image '{}' is {} days old", resource.name, age_days),
            "Old custom images accumulate storage costs. Delete if no longer needed.".into(),
            savings,
        ))
    } else {
        None
    }
}

fn check_sql_stopped(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    if !matches!(&resource.resource_type, crate::cloud::provider::ResourceType::CloudSqlInstance) {
        return None;
    }
    let status = resource.status.to_uppercase();
    if status == "STOPPED" || status == "SUSPENDED" {
        let savings = resource.monthly_cost.unwrap_or(0.0);
        let tier = resource.metadata.get("tier").and_then(|v| v.as_str()).unwrap_or("unknown");
        Some(make_finding(
            resource,
            rule,
            format!("Cloud SQL instance '{}' ({}) is {} — still incurs storage costs", resource.name, tier, status.to_lowercase()),
            "Delete this instance if no longer needed, or restart it. Stopped instances still pay for storage.".into(),
            savings * 0.3, // storage portion ~30% of total cost
        ))
    } else {
        None
    }
}

fn check_function_idle(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    if !matches!(&resource.resource_type, crate::cloud::provider::ResourceType::ServerlessFunction) {
        return None;
    }
    let min_instances = resource.metadata.get("min_instances").and_then(|v| v.as_i64()).unwrap_or(0);
    if min_instances > 0 {
        let savings = resource.monthly_cost.unwrap_or(0.0);
        Some(make_finding(
            resource,
            rule,
            format!("Cloud Function '{}' has {} min instances configured, incurring idle costs", resource.name, min_instances),
            "Set min instances to 0 if cold starts are acceptable. Min instances cost money even without invocations.".into(),
            savings,
        ))
    } else {
        None
    }
}

fn check_run_idle(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    if !matches!(&resource.resource_type, crate::cloud::provider::ResourceType::CloudRunService) {
        return None;
    }
    let min_instances = resource.metadata.get("min_instances").and_then(|v| v.as_i64()).unwrap_or(0);
    if min_instances > 0 {
        let savings = resource.monthly_cost.unwrap_or(0.0);
        Some(make_finding(
            resource,
            rule,
            format!("Cloud Run service '{}' has {} min instances, incurring idle costs", resource.name, min_instances),
            "Set min instances to 0 if cold starts are acceptable to eliminate idle costs.".into(),
            savings,
        ))
    } else {
        None
    }
}

fn check_untagged(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    use crate::cloud::provider::ResourceType;
    // Skip types that typically don't have user tags
    if matches!(&resource.resource_type,
        ResourceType::SecurityGroup | ResourceType::Network | ResourceType::LogSink |
        ResourceType::PubSubTopic | ResourceType::PubSubSubscription | ResourceType::NatGateway
    ) {
        return None;
    }
    if resource.tags.is_empty() {
        let type_name = serde_json::to_string(&resource.resource_type)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();
        Some(make_finding(
            resource,
            rule,
            format!("{} '{}' has no labels/tags", type_name, resource.name),
            "Add labels for cost allocation, ownership tracking, and environment identification.".into(),
            0.0,
        ))
    } else {
        None
    }
}

// --- New detection functions for added services ---

fn check_gke_degraded(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    if !matches!(&resource.resource_type, crate::cloud::provider::ResourceType::GkeCluster) {
        return None;
    }
    let status = resource.status.to_uppercase();
    if status == "ERROR" || status == "DEGRADED" {
        let savings = resource.monthly_cost.unwrap_or(0.0);
        Some(make_finding(resource, rule,
            format!("GKE cluster '{}' is in {} state", resource.name, status),
            "Investigate and repair this cluster, or delete if no longer needed.".into(),
            savings,
        ))
    } else { None }
}

fn check_gke_overprovisioned(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    if !matches!(&resource.resource_type, crate::cloud::provider::ResourceType::GkeCluster) {
        return None;
    }
    let node_count = resource.metadata.get("current_node_count").and_then(|v| v.as_i64()).unwrap_or(0);
    if node_count >= 10 {
        let savings = resource.monthly_cost.unwrap_or(0.0) * 0.3;
        Some(make_finding(resource, rule,
            format!("GKE cluster '{}' has {} nodes — review if all are needed", resource.name, node_count),
            "Consider enabling cluster autoscaler or reducing node pool size to match actual workload.".into(),
            savings,
        ))
    } else { None }
}

fn check_bq_empty(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    if !matches!(&resource.resource_type, crate::cloud::provider::ResourceType::BigQueryDataset) {
        return None;
    }
    if resource.status.to_uppercase() == "EMPTY" {
        Some(make_finding(resource, rule,
            format!("BigQuery dataset '{}' has no tables", resource.name),
            "Delete empty datasets to reduce clutter.".into(),
            0.0,
        ))
    } else { None }
}

fn check_bq_large(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    if !matches!(&resource.resource_type, crate::cloud::provider::ResourceType::BigQueryDataset) {
        return None;
    }
    let size_gb = resource.metadata.get("total_size_bytes")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) / (1024.0 * 1024.0 * 1024.0);
    if size_gb > 100.0 {
        let savings = resource.monthly_cost.unwrap_or(0.0) * 0.2;
        Some(make_finding(resource, rule,
            format!("BigQuery dataset '{}' is {:.1} GB — review table expiration and partitioning", resource.name, size_gb),
            "Set table expiration policies, use partitioning, and archive old data to reduce storage costs.".into(),
            savings,
        ))
    } else { None }
}

fn check_pubsub_detached(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    if !matches!(&resource.resource_type, crate::cloud::provider::ResourceType::PubSubSubscription) {
        return None;
    }
    if resource.status.to_uppercase() == "DETACHED" {
        Some(make_finding(resource, rule,
            format!("Pub/Sub subscription '{}' is detached — topic has been deleted", resource.name),
            "Delete this orphaned subscription as it will never receive new messages.".into(),
            0.0,
        ))
    } else { None }
}

fn check_spanner_idle(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    if !matches!(&resource.resource_type, crate::cloud::provider::ResourceType::SpannerInstance) {
        return None;
    }
    let savings = resource.monthly_cost.unwrap_or(0.0);
    if savings > 0.0 {
        Some(make_finding(resource, rule,
            format!("Spanner instance '{}' costs {}/month — verify it's actively used", resource.name, format!("${:.0}", savings)),
            "Cloud Spanner is very expensive. Scale down processing units or delete if not actively used.".into(),
            savings * 0.3,
        ))
    } else { None }
}

fn check_memorystore_idle(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    if !matches!(&resource.resource_type, crate::cloud::provider::ResourceType::MemorystoreInstance) {
        return None;
    }
    let status = resource.status.to_uppercase();
    if status == "RUNNING" || status == "READY" {
        let savings = resource.monthly_cost.unwrap_or(0.0);
        Some(make_finding(resource, rule,
            format!("Memorystore instance '{}' is running — verify it's actively used", resource.name),
            "Review if this Redis instance is still needed. Consider downscaling memory if underutilized.".into(),
            savings * 0.2,
        ))
    } else { None }
}

fn check_appengine_stopped(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    if !matches!(&resource.resource_type, crate::cloud::provider::ResourceType::AppEngineVersion) {
        return None;
    }
    if resource.status.to_uppercase() == "STOPPED" {
        Some(make_finding(resource, rule,
            format!("App Engine version '{}' is stopped but still deployed", resource.name),
            "Delete unused App Engine versions to clean up.".into(),
            0.0,
        ))
    } else { None }
}

fn check_nat_unused(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    if !matches!(&resource.resource_type, crate::cloud::provider::ResourceType::NatGateway) {
        return None;
    }
    let savings = resource.monthly_cost.unwrap_or(31.68);
    Some(make_finding(resource, rule,
        format!("Cloud NAT gateway '{}' costs ~${:.0}/month", resource.name, savings),
        "Review if this NAT gateway is still needed. Each gateway incurs hourly charges.".into(),
        savings,
    ))
}

fn check_vpn_down(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    if !matches!(&resource.resource_type, crate::cloud::provider::ResourceType::VpnTunnel) {
        return None;
    }
    let status = resource.status.to_uppercase();
    if status != "ESTABLISHED" {
        let savings = resource.monthly_cost.unwrap_or(54.0);
        Some(make_finding(resource, rule,
            format!("VPN tunnel '{}' is in {} state — not connected", resource.name, status),
            "Fix or delete this VPN tunnel. Non-established tunnels still incur costs.".into(),
            savings,
        ))
    } else { None }
}

fn check_artifact_large(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    if !matches!(&resource.resource_type, crate::cloud::provider::ResourceType::ArtifactRegistryRepo) {
        return None;
    }
    let size_gb = resource.metadata.get("size_bytes")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) / (1024.0 * 1024.0 * 1024.0);
    if size_gb > 10.0 {
        let savings = resource.monthly_cost.unwrap_or(0.0) * 0.3;
        Some(make_finding(resource, rule,
            format!("Artifact Registry repo '{}' is {:.1} GB", resource.name, size_gb),
            "Set up cleanup policies to remove old container images and reduce storage costs.".into(),
            savings,
        ))
    } else { None }
}

fn check_dataproc_idle(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    if !matches!(&resource.resource_type, crate::cloud::provider::ResourceType::DataprocCluster) {
        return None;
    }
    let status = resource.status.to_uppercase();
    if status == "RUNNING" {
        let savings = resource.monthly_cost.unwrap_or(0.0);
        Some(make_finding(resource, rule,
            format!("Dataproc cluster '{}' is running — verify it's actively processing jobs", resource.name),
            "Consider using ephemeral clusters (create per job, delete after). Long-running clusters waste money.".into(),
            savings * 0.5,
        ))
    } else { None }
}

fn check_secret_disabled(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    if !matches!(&resource.resource_type, crate::cloud::provider::ResourceType::SecretManagerSecret) {
        return None;
    }
    if resource.status.to_uppercase() == "DISABLED" {
        Some(make_finding(resource, rule,
            format!("Secret '{}' has no active versions", resource.name),
            "Delete unused secrets to reduce clutter and minor storage costs.".into(),
            resource.monthly_cost.unwrap_or(0.0),
        ))
    } else { None }
}

fn check_logsink_disabled(resource: &CloudResource, rule: &crate::analysis::rules::DetectionRule) -> Option<Finding> {
    if !matches!(&resource.resource_type, crate::cloud::provider::ResourceType::LogSink) {
        return None;
    }
    if resource.status.to_uppercase() == "DISABLED" {
        Some(make_finding(resource, rule,
            format!("Log sink '{}' is disabled", resource.name),
            "Delete disabled log sinks if they're no longer needed.".into(),
            0.0,
        ))
    } else { None }
}
