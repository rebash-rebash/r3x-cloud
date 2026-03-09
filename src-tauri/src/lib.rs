use tauri::Manager;

mod analysis;
mod cloud;
mod cost;
mod credentials;
mod export;
mod scanner;
mod storage;

use storage::db::Database;

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
            // Scanning
            scanner::engine::start_scan,
            scanner::engine::get_scan_resources,
            scanner::engine::get_latest_resources,
            // Analysis rules
            analysis::rules::list_rules,
        ])
        .run(tauri::generate_context!())
        .expect("error while running r3x-cloud");
}
