use crate::cloud::gcp::provider::GcpProvider;
use crate::cloud::provider::*;
use crate::storage::db::Database;
use std::sync::Arc;
use tauri::{Emitter, State};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScanResult {
    pub scan_id: String,
    pub account_id: String,
    pub total_resources: usize,
    pub status: String,
}

#[tauri::command]
pub async fn start_scan(
    account_id: String,
    db: State<'_, Database>,
    app: tauri::AppHandle,
) -> Result<ScanResult, String> {
    // Load account from DB
    let accounts = db.list_accounts().map_err(|e| e.to_string())?;
    let account = accounts
        .iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| format!("Account {} not found", account_id))?
        .clone();

    let scan_id = uuid::Uuid::new_v4().to_string();
    db.create_scan(&scan_id, &account_id)
        .map_err(|e| e.to_string())?;

    // Build provider
    let provider: Arc<dyn CloudProvider> = match account.provider {
        ProviderKind::Gcp => {
            let gcp = GcpProvider::new(
                account.project_id.clone().unwrap_or_default(),
                account.config.clone(),
            )
            .map_err(|e| e.to_string())?;
            Arc::new(gcp)
        }
        _ => return Err(format!("Provider {:?} not yet supported", account.provider)),
    };

    // Using aggregatedList APIs, so we scan per resource type (not per zone)
    let resource_types = provider.supported_resource_types();
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel::<ScanProgress>(100);

    // Spawn progress forwarder to frontend
    let app_clone = app.clone();
    tokio::spawn(async move {
        while let Some(progress) = progress_rx.recv().await {
            let _ = app_clone.emit("scan-progress", &progress);
        }
    });

    // Scan all resource types in parallel
    let semaphore = Arc::new(tokio::sync::Semaphore::new(5));
    let mut handles = Vec::new();

    for rt in &resource_types {
        let provider = provider.clone();
        let rt = rt.clone();
        let tx = progress_tx.clone();
        let sem = semaphore.clone();

        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            match provider.scan_resource_type("aggregated", &rt, tx).await {
                Ok(resources) => resources,
                Err(e) => {
                    tracing::error!("Scan error for {:?}: {}", rt, e);
                    vec![]
                }
            }
        }));
    }

    drop(progress_tx); // Close sender so the forwarder task can complete

    // Collect results
    let mut all_resources = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(resources) => all_resources.extend(resources),
            Err(e) => tracing::error!("Task join error: {}", e),
        }
    }

    let total = all_resources.len();

    // Store in DB
    db.insert_resources(&scan_id, &all_resources)
        .map_err(|e| e.to_string())?;
    db.complete_scan(&scan_id, total)
        .map_err(|e| e.to_string())?;

    // Emit completion event
    let _ = app.emit("scan-complete", serde_json::json!({
        "scan_id": scan_id,
        "total_resources": total,
    }));

    Ok(ScanResult {
        scan_id,
        account_id,
        total_resources: total,
        status: "completed".to_string(),
    })
}

#[tauri::command]
pub async fn get_scan_resources(
    scan_id: String,
    db: State<'_, Database>,
) -> Result<Vec<CloudResource>, String> {
    db.get_scan_resources(&scan_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_latest_resources(
    account_id: String,
    db: State<'_, Database>,
) -> Result<Vec<CloudResource>, String> {
    let scan_id = db
        .get_latest_scan_id(&account_id)
        .map_err(|e| e.to_string())?;

    match scan_id {
        Some(id) => db.get_scan_resources(&id).map_err(|e| e.to_string()),
        None => Ok(vec![]),
    }
}
