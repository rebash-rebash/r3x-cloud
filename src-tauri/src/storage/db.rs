use crate::cloud::provider::{CloudAccount, CloudResource, ProviderKind};
use rusqlite::{params, Connection};
use std::sync::Mutex;
use tauri::Manager;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(app_handle: &tauri::AppHandle) -> anyhow::Result<Self> {
        let app_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| anyhow::anyhow!("Failed to get app data dir: {}", e))?;

        std::fs::create_dir_all(&app_dir)?;
        let db_path = app_dir.join("r3x-cloud.db");

        tracing::info!("Database path: {:?}", db_path);

        let conn = Connection::open(db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.initialize_schema()?;
        Ok(db)
    }

    fn initialize_schema(&self) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS accounts (
                id TEXT PRIMARY KEY,
                provider TEXT NOT NULL,
                display_name TEXT NOT NULL,
                project_id TEXT,
                config_json TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS scans (
                id TEXT PRIMARY KEY,
                account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
                started_at TEXT NOT NULL DEFAULT (datetime('now')),
                completed_at TEXT,
                status TEXT NOT NULL DEFAULT 'running',
                resource_count INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS resources (
                id TEXT NOT NULL,
                scan_id TEXT NOT NULL REFERENCES scans(id) ON DELETE CASCADE,
                account_id TEXT NOT NULL,
                provider TEXT NOT NULL,
                resource_type TEXT NOT NULL,
                region TEXT NOT NULL,
                name TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT,
                last_used TEXT,
                tags_json TEXT NOT NULL DEFAULT '{}',
                metadata_json TEXT NOT NULL DEFAULT '{}',
                monthly_cost REAL,
                PRIMARY KEY (id, scan_id)
            );

            CREATE TABLE IF NOT EXISTS findings (
                id TEXT PRIMARY KEY,
                scan_id TEXT NOT NULL REFERENCES scans(id) ON DELETE CASCADE,
                resource_id TEXT NOT NULL,
                rule_id TEXT NOT NULL,
                severity TEXT NOT NULL,
                description TEXT NOT NULL,
                estimated_monthly_savings REAL,
                recommendation TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS rule_configs (
                rule_id TEXT PRIMARY KEY,
                enabled INTEGER NOT NULL DEFAULT 1,
                threshold_json TEXT NOT NULL DEFAULT '{}'
            );
            ",
        )?;
        Ok(())
    }

    // --- Account operations ---

    pub fn list_accounts(&self) -> anyhow::Result<Vec<CloudAccount>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT id, provider, display_name, project_id, config_json FROM accounts ORDER BY created_at")?;
        let accounts = stmt
            .query_map([], |row| {
                let provider_str: String = row.get(1)?;
                let config_str: String = row.get(4)?;
                Ok(CloudAccount {
                    id: row.get(0)?,
                    provider: match provider_str.as_str() {
                        "gcp" => ProviderKind::Gcp,
                        "aws" => ProviderKind::Aws,
                        "azure" => ProviderKind::Azure,
                        _ => ProviderKind::Gcp,
                    },
                    display_name: row.get(2)?,
                    project_id: row.get(3)?,
                    config: serde_json::from_str(&config_str).unwrap_or_default(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(accounts)
    }

    pub fn insert_account(&self, account: &CloudAccount) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO accounts (id, provider, display_name, project_id, config_json) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                account.id,
                account.provider.to_string(),
                account.display_name,
                account.project_id,
                serde_json::to_string(&account.config)?,
            ],
        )?;
        Ok(())
    }

    pub fn delete_account(&self, id: &str) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM accounts WHERE id = ?1", params![id])?;
        Ok(())
    }

    // --- Scan operations ---

    pub fn create_scan(&self, scan_id: &str, account_id: &str) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO scans (id, account_id, status) VALUES (?1, ?2, 'running')",
            params![scan_id, account_id],
        )?;
        Ok(())
    }

    pub fn complete_scan(&self, scan_id: &str, resource_count: usize) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE scans SET status = 'completed', completed_at = datetime('now'), resource_count = ?1 WHERE id = ?2",
            params![resource_count as i64, scan_id],
        )?;
        Ok(())
    }

    pub fn fail_scan(&self, scan_id: &str) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE scans SET status = 'failed', completed_at = datetime('now') WHERE id = ?1",
            params![scan_id],
        )?;
        Ok(())
    }

    // --- Resource operations ---

    pub fn insert_resources(&self, scan_id: &str, resources: &[CloudResource]) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "INSERT OR REPLACE INTO resources (id, scan_id, account_id, provider, resource_type, region, name, status, created_at, last_used, tags_json, metadata_json, monthly_cost)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        )?;

        for r in resources {
            stmt.execute(params![
                r.id,
                scan_id,
                r.account_id,
                r.provider.to_string(),
                serde_json::to_string(&r.resource_type)?,
                r.region,
                r.name,
                r.status,
                r.created_at,
                r.last_used,
                serde_json::to_string(&r.tags)?,
                serde_json::to_string(&r.metadata)?,
                r.monthly_cost,
            ])?;
        }
        Ok(())
    }

    pub fn get_scan_resources(&self, scan_id: &str) -> anyhow::Result<Vec<CloudResource>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, account_id, provider, resource_type, region, name, status, created_at, last_used, tags_json, metadata_json, monthly_cost FROM resources WHERE scan_id = ?1",
        )?;
        let resources = stmt
            .query_map(params![scan_id], |row| {
                let provider_str: String = row.get(2)?;
                let resource_type_str: String = row.get(3)?;
                let tags_str: String = row.get(9)?;
                let metadata_str: String = row.get(10)?;

                Ok(CloudResource {
                    id: row.get(0)?,
                    name: row.get(5)?,
                    resource_type: serde_json::from_str(&resource_type_str)
                        .unwrap_or(crate::cloud::provider::ResourceType::VirtualMachine),
                    provider: match provider_str.as_str() {
                        "gcp" => ProviderKind::Gcp,
                        "aws" => ProviderKind::Aws,
                        "azure" => ProviderKind::Azure,
                        _ => ProviderKind::Gcp,
                    },
                    region: row.get(4)?,
                    account_id: row.get(1)?,
                    status: row.get(6)?,
                    created_at: row.get(7)?,
                    last_used: row.get(8)?,
                    tags: serde_json::from_str(&tags_str).unwrap_or_default(),
                    metadata: serde_json::from_str(&metadata_str).unwrap_or_default(),
                    monthly_cost: row.get(11)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(resources)
    }

    pub fn get_latest_scan_id(&self, account_id: &str) -> anyhow::Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT id FROM scans WHERE account_id = ?1 AND status = 'completed' ORDER BY completed_at DESC LIMIT 1",
            params![account_id],
            |row| row.get(0),
        );
        match result {
            Ok(id) => Ok(Some(id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
