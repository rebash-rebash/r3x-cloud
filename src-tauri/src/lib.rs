use tauri::Manager;

mod analysis;
mod cloud;
mod cost;
mod credentials;
mod export;
mod scanner;
mod storage;

use analysis::detector::AnalysisSummary;
use storage::db::{Database, ScanRecord};

#[tauri::command]
async fn run_analysis(
    account_id: String,
    db: tauri::State<'_, Database>,
) -> Result<AnalysisSummary, String> {
    let scan_id = db
        .get_latest_scan_id(&account_id)
        .map_err(|e| e.to_string())?;

    let resources = match scan_id {
        Some(id) => db.get_scan_resources(&id).map_err(|e| e.to_string())?,
        None => return Err("No scan data available. Run a scan first.".into()),
    };

    // Load saved rule configs so toggled rules are respected
    let rule_overrides: std::collections::HashMap<String, bool> = db
        .get_rule_configs()
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();

    Ok(analysis::detector::analyze_resources(&resources, &rule_overrides))
}

#[tauri::command]
async fn list_scans(
    account_id: String,
    db: tauri::State<'_, Database>,
) -> Result<Vec<ScanRecord>, String> {
    db.list_scans(&account_id).map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
struct RuleConfigInput {
    rule_id: String,
    enabled: bool,
}

#[tauri::command]
async fn save_rule_configs(
    configs: Vec<RuleConfigInput>,
    db: tauri::State<'_, Database>,
) -> Result<(), String> {
    let pairs: Vec<(String, bool)> = configs.into_iter().map(|c| (c.rule_id, c.enabled)).collect();
    db.save_rule_configs(&pairs).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_rule_configs(
    db: tauri::State<'_, Database>,
) -> Result<Vec<(String, bool)>, String> {
    db.get_rule_configs().map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
struct CostTrendPoint {
    scan_id: String,
    completed_at: String,
    total_monthly_cost: f64,
    resource_count: i64,
}

#[tauri::command]
async fn get_cost_trend(
    account_id: String,
    db: tauri::State<'_, Database>,
) -> Result<Vec<CostTrendPoint>, String> {
    let trend = db.get_cost_trend(&account_id).map_err(|e| e.to_string())?;
    Ok(trend
        .into_iter()
        .map(|(scan_id, completed_at, total_monthly_cost, resource_count)| CostTrendPoint {
            scan_id,
            completed_at,
            total_monthly_cost,
            resource_count,
        })
        .collect())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let db = Database::new(&app.handle())?;
            app.manage(db);
            tracing::info!("r3x-cloud initialized");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Account management
            cloud::account::list_accounts,
            cloud::account::add_account,
            cloud::account::remove_account,
            cloud::account::test_connection,
            cloud::account::list_gcp_projects,
            // Scanning
            scanner::engine::start_scan,
            scanner::engine::get_scan_resources,
            scanner::engine::get_latest_resources,
            // Analysis
            analysis::rules::list_rules,
            run_analysis,
            list_scans,
            // Settings
            save_rule_configs,
            get_rule_configs,
            // Trend
            get_cost_trend,
            // Export
            export::export_to_file,
            // Cost
            cost::get_pricing_data,
            // Credentials
            credentials::check_credentials,
        ])
        .run(tauri::generate_context!())
        .expect("error while running r3x-cloud");
}
