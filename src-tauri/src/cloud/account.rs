use crate::cloud::provider::{CloudAccount, CloudProvider, ProviderKind};
use crate::storage::db::Database;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcpProject {
    pub project_id: String,
    pub name: String,
    pub state: String,
}

#[tauri::command]
pub async fn list_accounts(db: State<'_, Database>) -> Result<Vec<CloudAccount>, String> {
    db.list_accounts().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_account(
    provider: ProviderKind,
    display_name: String,
    project_id: Option<String>,
    config: serde_json::Value,
    db: State<'_, Database>,
) -> Result<CloudAccount, String> {
    let account = CloudAccount {
        id: uuid::Uuid::new_v4().to_string(),
        provider,
        display_name,
        project_id,
        config,
    };
    db.insert_account(&account).map_err(|e| e.to_string())?;
    Ok(account)
}

#[tauri::command]
pub async fn remove_account(id: String, db: State<'_, Database>) -> Result<(), String> {
    db.delete_account(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_connection(
    provider: ProviderKind,
    project_id: Option<String>,
    config: serde_json::Value,
) -> Result<String, String> {
    match provider {
        ProviderKind::Gcp => {
            let gcp = crate::cloud::gcp::provider::GcpProvider::new(
                project_id.unwrap_or_default(),
                config,
            )
            .map_err(|e| e.to_string())?;
            gcp.validate_credentials()
                .await
                .map_err(|e| e.to_string())
        }
        _ => Err(format!("Provider {:?} not yet supported", provider)),
    }
}

/// List all GCP projects accessible to the authenticated user via gcloud CLI.
#[tauri::command]
pub async fn list_gcp_projects() -> Result<Vec<GcpProject>, String> {
    let gcloud_path = crate::cloud::gcp::provider::find_gcloud().ok_or_else(|| {
        "Could not find gcloud CLI. Ensure Google Cloud SDK is installed.".to_string()
    })?;

    let output = tokio::process::Command::new(&gcloud_path)
        .args(["projects", "list", "--format=json", "--sort-by=projectId"])
        .output()
        .await
        .map_err(|e| format!("Failed to run gcloud: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gcloud projects list failed: {}", stderr.trim()));
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Failed to parse gcloud output: {}", e))?;

    let projects = json
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|p| {
                    let project_id = p["projectId"].as_str()?.to_string();
                    let name = p["name"].as_str().unwrap_or(&project_id).to_string();
                    let state = p["lifecycleState"].as_str().unwrap_or("ACTIVE").to_string();
                    if state == "ACTIVE" {
                        Some(GcpProject { project_id, name, state })
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(projects)
}
