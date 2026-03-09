use crate::cloud::provider::CloudResource;
use crate::analysis::detector::Finding;
use std::path::PathBuf;

/// Export resources to CSV file on disk.
pub fn export_resources_csv(resources: &[CloudResource], path: &PathBuf) -> anyhow::Result<String> {
    let mut wtr = csv::Writer::from_path(path)?;

    wtr.write_record(["Name", "Type", "Status", "Region", "Provider", "Monthly Cost", "Created", "ID", "Tags"])?;

    for r in resources {
        let type_str = serde_json::to_string(&r.resource_type)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();
        let cost_str = r.monthly_cost.map(|c| format!("{:.2}", c)).unwrap_or_default();
        let tags_str = r.tags.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("; ");

        wtr.write_record([
            &r.name,
            &type_str,
            &r.status,
            &r.region,
            &r.provider.to_string(),
            &cost_str,
            r.created_at.as_deref().unwrap_or(""),
            &r.id,
            &tags_str,
        ])?;
    }

    wtr.flush()?;
    Ok(path.to_string_lossy().to_string())
}

/// Export findings to CSV file on disk.
pub fn export_findings_csv(findings: &[Finding], path: &PathBuf) -> anyhow::Result<String> {
    let mut wtr = csv::Writer::from_path(path)?;

    wtr.write_record(["Resource", "Type", "Rule", "Severity", "Description", "Recommendation", "Monthly Savings", "Region"])?;

    for f in findings {
        let severity = serde_json::to_string(&f.severity)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();

        wtr.write_record([
            &f.resource_name,
            &f.resource_type,
            &f.rule_name,
            &severity,
            &f.description,
            &f.recommendation,
            &format!("{:.2}", f.estimated_monthly_savings),
            &f.region,
        ])?;
    }

    wtr.flush()?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn export_to_file(
    account_id: String,
    format: String,
    export_type: String,
    db: tauri::State<'_, crate::storage::db::Database>,
) -> Result<String, String> {
    let scan_id = db.get_latest_scan_id(&account_id)
        .map_err(|e| e.to_string())?
        .ok_or("No scan data available")?;

    let resources = db.get_scan_resources(&scan_id).map_err(|e| e.to_string())?;

    let rule_overrides: std::collections::HashMap<String, bool> = db
        .get_rule_configs()
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();

    // Use Downloads folder
    let home = std::env::var("HOME").unwrap_or_default();
    let downloads = PathBuf::from(&home).join("Downloads");

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");

    match (format.as_str(), export_type.as_str()) {
        ("csv", "resources") => {
            let path = downloads.join(format!("r3x_resources_{}.csv", timestamp));
            export_resources_csv(&resources, &path).map_err(|e| e.to_string())
        }
        ("csv", "findings") => {
            let analysis = crate::analysis::detector::analyze_resources(&resources, &rule_overrides);
            let path = downloads.join(format!("r3x_findings_{}.csv", timestamp));
            export_findings_csv(&analysis.findings, &path).map_err(|e| e.to_string())
        }
        ("json", "resources") => {
            let path = downloads.join(format!("r3x_resources_{}.json", timestamp));
            let json = serde_json::to_string_pretty(&resources).map_err(|e| e.to_string())?;
            std::fs::write(&path, json).map_err(|e| e.to_string())?;
            Ok(path.to_string_lossy().to_string())
        }
        ("json", "findings") => {
            let analysis = crate::analysis::detector::analyze_resources(&resources, &rule_overrides);
            let path = downloads.join(format!("r3x_findings_{}.json", timestamp));
            let json = serde_json::to_string_pretty(&analysis).map_err(|e| e.to_string())?;
            std::fs::write(&path, json).map_err(|e| e.to_string())?;
            Ok(path.to_string_lossy().to_string())
        }
        ("json", "all") => {
            let analysis = crate::analysis::detector::analyze_resources(&resources, &rule_overrides);
            let path = downloads.join(format!("r3x_report_{}.json", timestamp));
            let report = serde_json::json!({
                "resources": resources,
                "analysis": analysis,
                "exported_at": chrono::Utc::now().to_rfc3339(),
            });
            let json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
            std::fs::write(&path, json).map_err(|e| e.to_string())?;
            Ok(path.to_string_lossy().to_string())
        }
        _ => Err("Invalid export format/type combination".into()),
    }
}
