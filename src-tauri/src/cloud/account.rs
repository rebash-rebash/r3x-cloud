use crate::cloud::provider::{CloudAccount, CloudProvider, ProviderKind};
use crate::storage::db::Database;
use tauri::State;

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
