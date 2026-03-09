use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub resource_type: String,
    pub severity: Severity,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

#[tauri::command]
pub fn list_rules() -> Vec<DetectionRule> {
    all_gcp_rules()
}

pub fn all_gcp_rules() -> Vec<DetectionRule> {
    vec![
        DetectionRule {
            id: "gcp-vm-stopped".into(),
            name: "Stopped VMs".into(),
            description: "Compute instances in TERMINATED state".into(),
            resource_type: "virtual_machine".into(),
            severity: Severity::High,
            enabled: true,
        },
        DetectionRule {
            id: "gcp-disk-unattached".into(),
            name: "Unattached Disks".into(),
            description: "Persistent disks not attached to any instance".into(),
            resource_type: "disk".into(),
            severity: Severity::High,
            enabled: true,
        },
        DetectionRule {
            id: "gcp-snapshot-old".into(),
            name: "Old Snapshots".into(),
            description: "Disk snapshots older than 90 days".into(),
            resource_type: "snapshot".into(),
            severity: Severity::Medium,
            enabled: true,
        },
        DetectionRule {
            id: "gcp-ip-unused".into(),
            name: "Unused Static IPs".into(),
            description: "Static external IPs in RESERVED state (not in use)".into(),
            resource_type: "elastic_ip".into(),
            severity: Severity::High,
            enabled: true,
        },
        DetectionRule {
            id: "gcp-lb-no-target".into(),
            name: "Load Balancers Without Targets".into(),
            description: "Forwarding rules with no backend target configured".into(),
            resource_type: "load_balancer".into(),
            severity: Severity::Medium,
            enabled: true,
        },
        DetectionRule {
            id: "gcp-fw-disabled".into(),
            name: "Disabled Firewall Rules".into(),
            description: "Firewall rules that are disabled and may be unused".into(),
            resource_type: "security_group".into(),
            severity: Severity::Low,
            enabled: true,
        },
        DetectionRule {
            id: "gcp-image-old".into(),
            name: "Old Machine Images".into(),
            description: "Custom images older than 180 days".into(),
            resource_type: "machine_image".into(),
            severity: Severity::Low,
            enabled: true,
        },
        DetectionRule {
            id: "gcp-disk-large-unused".into(),
            name: "Large Unattached Disks".into(),
            description: "Unattached disks larger than 100GB".into(),
            resource_type: "disk".into(),
            severity: Severity::Critical,
            enabled: true,
        },
        DetectionRule {
            id: "gcp-sql-stopped".into(),
            name: "Stopped Cloud SQL Instances".into(),
            description: "Cloud SQL instances in STOPPED or SUSPENDED state still incur storage costs".into(),
            resource_type: "cloud_sql_instance".into(),
            severity: Severity::High,
            enabled: true,
        },
        DetectionRule {
            id: "gcp-function-idle".into(),
            name: "Idle Cloud Functions".into(),
            description: "Cloud Functions with min instances > 0 but no recent updates".into(),
            resource_type: "serverless_function".into(),
            severity: Severity::Medium,
            enabled: true,
        },
        DetectionRule {
            id: "gcp-run-idle".into(),
            name: "Idle Cloud Run Services".into(),
            description: "Cloud Run services with min instances > 0 incurring idle costs".into(),
            resource_type: "cloud_run_service".into(),
            severity: Severity::Medium,
            enabled: true,
        },
        DetectionRule {
            id: "gcp-resource-untagged".into(),
            name: "Untagged Resources".into(),
            description: "Resources with no labels/tags for cost allocation and ownership tracking".into(),
            resource_type: "*".into(),
            severity: Severity::Medium,
            enabled: true,
        },
        // --- GKE ---
        DetectionRule {
            id: "gcp-gke-degraded".into(),
            name: "Degraded GKE Clusters".into(),
            description: "GKE clusters in ERROR or DEGRADED state".into(),
            resource_type: "gke_cluster".into(),
            severity: Severity::Critical,
            enabled: true,
        },
        DetectionRule {
            id: "gcp-gke-overprovisioned".into(),
            name: "Overprovisioned GKE Clusters".into(),
            description: "GKE clusters with 10+ nodes that may be overprovisioned".into(),
            resource_type: "gke_cluster".into(),
            severity: Severity::High,
            enabled: true,
        },
        // --- BigQuery ---
        DetectionRule {
            id: "gcp-bq-empty".into(),
            name: "Empty BigQuery Datasets".into(),
            description: "BigQuery datasets with no tables".into(),
            resource_type: "big_query_dataset".into(),
            severity: Severity::Low,
            enabled: true,
        },
        DetectionRule {
            id: "gcp-bq-large".into(),
            name: "Large BigQuery Datasets".into(),
            description: "BigQuery datasets over 100GB in storage".into(),
            resource_type: "big_query_dataset".into(),
            severity: Severity::Medium,
            enabled: true,
        },
        // --- Pub/Sub ---
        DetectionRule {
            id: "gcp-pubsub-detached".into(),
            name: "Detached Pub/Sub Subscriptions".into(),
            description: "Subscriptions whose topic has been deleted".into(),
            resource_type: "pub_sub_subscription".into(),
            severity: Severity::High,
            enabled: true,
        },
        // --- Spanner ---
        DetectionRule {
            id: "gcp-spanner-idle".into(),
            name: "Idle Spanner Instances".into(),
            description: "Cloud Spanner instances are very expensive; review if still needed".into(),
            resource_type: "spanner_instance".into(),
            severity: Severity::High,
            enabled: true,
        },
        // --- Memorystore ---
        DetectionRule {
            id: "gcp-memorystore-idle".into(),
            name: "Idle Memorystore Instances".into(),
            description: "Memorystore (Redis) instances that may be unused".into(),
            resource_type: "memorystore_instance".into(),
            severity: Severity::Medium,
            enabled: true,
        },
        // --- App Engine ---
        DetectionRule {
            id: "gcp-appengine-stopped".into(),
            name: "Stopped App Engine Versions".into(),
            description: "App Engine versions that are stopped but still deployed".into(),
            resource_type: "app_engine_version".into(),
            severity: Severity::Low,
            enabled: true,
        },
        // --- NAT ---
        DetectionRule {
            id: "gcp-nat-unused".into(),
            name: "Cloud NAT Gateways".into(),
            description: "Cloud NAT gateways incur hourly costs; review if still needed".into(),
            resource_type: "nat_gateway".into(),
            severity: Severity::Low,
            enabled: true,
        },
        // --- VPN ---
        DetectionRule {
            id: "gcp-vpn-down".into(),
            name: "VPN Tunnels Not Established".into(),
            description: "VPN tunnels that are not in ESTABLISHED state".into(),
            resource_type: "vpn_tunnel".into(),
            severity: Severity::High,
            enabled: true,
        },
        // --- Artifact Registry ---
        DetectionRule {
            id: "gcp-artifact-large".into(),
            name: "Large Artifact Registry Repos".into(),
            description: "Artifact Registry repositories over 10GB in storage".into(),
            resource_type: "artifact_registry_repo".into(),
            severity: Severity::Medium,
            enabled: true,
        },
        // --- Dataproc ---
        DetectionRule {
            id: "gcp-dataproc-idle".into(),
            name: "Idle Dataproc Clusters".into(),
            description: "Dataproc clusters that are running but may be idle".into(),
            resource_type: "dataproc_cluster".into(),
            severity: Severity::High,
            enabled: true,
        },
        // --- Secret Manager ---
        DetectionRule {
            id: "gcp-secret-disabled".into(),
            name: "Disabled Secrets".into(),
            description: "Secret Manager secrets with all versions disabled or destroyed".into(),
            resource_type: "secret_manager_secret".into(),
            severity: Severity::Low,
            enabled: true,
        },
        // --- Logging ---
        DetectionRule {
            id: "gcp-logsink-disabled".into(),
            name: "Disabled Log Sinks".into(),
            description: "Log sinks that are disabled and may be unused".into(),
            resource_type: "log_sink".into(),
            severity: Severity::Low,
            enabled: true,
        },
    ]
}
