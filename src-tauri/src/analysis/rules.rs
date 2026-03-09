use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionRule {
    pub id: String,
    pub name: String,
    pub description: String,
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
    vec![
        DetectionRule {
            id: "gcp-vm-stopped".to_string(),
            name: "Stopped VMs".to_string(),
            description: "Compute instances in TERMINATED state for more than 7 days".to_string(),
            severity: Severity::High,
            enabled: true,
        },
        DetectionRule {
            id: "gcp-vm-low-cpu".to_string(),
            name: "Low CPU Utilization".to_string(),
            description: "VMs with average CPU below 5% over 14 days".to_string(),
            severity: Severity::Medium,
            enabled: true,
        },
        DetectionRule {
            id: "gcp-disk-unattached".to_string(),
            name: "Unattached Disks".to_string(),
            description: "Persistent disks not attached to any instance".to_string(),
            severity: Severity::High,
            enabled: true,
        },
        DetectionRule {
            id: "gcp-snapshot-old".to_string(),
            name: "Old Snapshots".to_string(),
            description: "Disk snapshots older than 90 days".to_string(),
            severity: Severity::Medium,
            enabled: true,
        },
        DetectionRule {
            id: "gcp-ip-unused".to_string(),
            name: "Unused Static IPs".to_string(),
            description: "Static external IPs in RESERVED state (not in use)".to_string(),
            severity: Severity::Medium,
            enabled: true,
        },
    ]
}
