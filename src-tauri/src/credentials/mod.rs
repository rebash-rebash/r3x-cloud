use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialStatus {
    pub provider: String,
    pub authenticated: bool,
    pub identity: String,
    pub method: String,
}

/// Check the current GCP authentication status.
#[tauri::command]
pub async fn check_credentials(provider: String) -> Result<CredentialStatus, String> {
    match provider.as_str() {
        "gcp" => check_gcp_credentials().await,
        _ => Err(format!("Provider '{}' not supported", provider)),
    }
}

async fn check_gcp_credentials() -> Result<CredentialStatus, String> {
    let gcloud_path = crate::cloud::gcp::provider::find_gcloud()
        .ok_or("gcloud CLI not found")?;

    // Check active account
    let output = tokio::process::Command::new(&gcloud_path)
        .args(["auth", "list", "--format=json"])
        .output()
        .await
        .map_err(|e| format!("Failed to run gcloud: {}", e))?;

    if !output.status.success() {
        return Ok(CredentialStatus {
            provider: "gcp".into(),
            authenticated: false,
            identity: "none".into(),
            method: "gcloud CLI".into(),
        });
    }

    let accounts: serde_json::Value = serde_json::from_slice(&output.stdout)
        .unwrap_or(serde_json::Value::Array(vec![]));

    let active = accounts.as_array()
        .and_then(|arr| arr.iter().find(|a| a["status"].as_str() == Some("ACTIVE")))
        .and_then(|a| a["account"].as_str())
        .unwrap_or("unknown");

    Ok(CredentialStatus {
        provider: "gcp".into(),
        authenticated: active != "unknown",
        identity: active.to_string(),
        method: "gcloud CLI".into(),
    })
}
