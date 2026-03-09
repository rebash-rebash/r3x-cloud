use serde::{Deserialize, Serialize};

/// Pricing data for common GCP resource types.
/// These are approximate on-demand prices for us-central1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingEntry {
    pub resource_type: String,
    pub sku: String,
    pub unit: String,
    pub price_per_unit: f64,
    pub region: String,
}

/// Get embedded pricing data for GCP resources.
#[tauri::command]
pub fn get_pricing_data() -> Vec<PricingEntry> {
    vec![
        // Compute Engine - VMs (per month, on-demand, us-central1)
        entry("virtual_machine", "e2-micro", "instance/month", 7.67),
        entry("virtual_machine", "e2-small", "instance/month", 15.33),
        entry("virtual_machine", "e2-medium", "instance/month", 30.67),
        entry("virtual_machine", "e2-standard-2", "instance/month", 61.34),
        entry("virtual_machine", "e2-standard-4", "instance/month", 122.67),
        entry("virtual_machine", "e2-standard-8", "instance/month", 245.35),
        entry("virtual_machine", "e2-standard-16", "instance/month", 490.69),
        entry("virtual_machine", "e2-highmem-2", "instance/month", 82.77),
        entry("virtual_machine", "e2-highmem-4", "instance/month", 165.54),
        entry("virtual_machine", "e2-highcpu-2", "instance/month", 45.63),
        entry("virtual_machine", "e2-highcpu-4", "instance/month", 91.25),
        entry("virtual_machine", "n1-standard-1", "instance/month", 34.67),
        entry("virtual_machine", "n1-standard-2", "instance/month", 69.35),
        entry("virtual_machine", "n1-standard-4", "instance/month", 138.70),
        entry("virtual_machine", "n1-standard-8", "instance/month", 277.40),
        entry("virtual_machine", "n2-standard-2", "instance/month", 71.54),
        entry("virtual_machine", "n2-standard-4", "instance/month", 143.08),
        entry("virtual_machine", "n2-standard-8", "instance/month", 286.16),
        entry("virtual_machine", "n2d-standard-2", "instance/month", 62.19),
        entry("virtual_machine", "n2d-standard-4", "instance/month", 124.38),

        // Persistent Disks (per GB/month)
        entry("disk", "pd-standard", "GB/month", 0.040),
        entry("disk", "pd-balanced", "GB/month", 0.100),
        entry("disk", "pd-ssd", "GB/month", 0.170),
        entry("disk", "pd-extreme", "GB/month", 0.125),
        entry("disk", "hyperdisk-balanced", "GB/month", 0.060),

        // Snapshots
        entry("snapshot", "snapshot-storage", "GB/month", 0.026),

        // Static IPs
        entry("elastic_ip", "unused-ip", "IP/month", 7.30),
        entry("elastic_ip", "in-use-ip", "IP/month", 0.0),

        // Load Balancers
        entry("load_balancer", "forwarding-rule", "rule/month", 18.26),

        // Cloud SQL (per month, on-demand)
        entry("cloud_sql_instance", "db-f1-micro", "instance/month", 7.67),
        entry("cloud_sql_instance", "db-g1-small", "instance/month", 25.55),
        entry("cloud_sql_instance", "db-n1-standard-1", "instance/month", 51.10),
        entry("cloud_sql_instance", "db-n1-standard-2", "instance/month", 102.20),
        entry("cloud_sql_instance", "db-n1-standard-4", "instance/month", 204.40),
        entry("cloud_sql_instance", "db-n1-standard-8", "instance/month", 408.80),
        entry("cloud_sql_instance", "ssd-storage", "GB/month", 0.170),
        entry("cloud_sql_instance", "hdd-storage", "GB/month", 0.090),

        // Cloud Storage (per GB/month)
        entry("storage_bucket", "STANDARD", "GB/month", 0.020),
        entry("storage_bucket", "NEARLINE", "GB/month", 0.010),
        entry("storage_bucket", "COLDLINE", "GB/month", 0.004),
        entry("storage_bucket", "ARCHIVE", "GB/month", 0.0012),

        // Cloud Functions
        entry("serverless_function", "invocations", "per million", 0.40),
        entry("serverless_function", "compute-128mb", "GB-seconds/month", 0.000000231),
        entry("serverless_function", "min-instance", "instance/month", 5.40),

        // Cloud Run
        entry("cloud_run_service", "cpu", "vCPU-second", 0.00002400),
        entry("cloud_run_service", "memory", "GiB-second", 0.00000250),
        entry("cloud_run_service", "min-instance", "instance/month", 15.00),
    ]
}

fn entry(resource_type: &str, sku: &str, unit: &str, price: f64) -> PricingEntry {
    PricingEntry {
        resource_type: resource_type.to_string(),
        sku: sku.to_string(),
        unit: unit.to_string(),
        price_per_unit: price,
        region: "us-central1".to_string(),
    }
}
