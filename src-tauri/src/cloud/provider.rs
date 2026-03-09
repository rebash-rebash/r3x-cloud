use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Gcp,
    Aws,
    Azure,
}

impl std::fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderKind::Gcp => write!(f, "gcp"),
            ProviderKind::Aws => write!(f, "aws"),
            ProviderKind::Azure => write!(f, "azure"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    VirtualMachine,
    Disk,
    Snapshot,
    LoadBalancer,
    ElasticIp,
    SecurityGroup,
    ServerlessFunction,
    StorageBucket,
    MachineImage,
    CloudSqlInstance,
    CloudRunService,
    Network,
    NatGateway,
    VpnTunnel,
    ArtifactRegistryRepo,
    DataprocCluster,
    SecretManagerSecret,
    LogSink,
    GkeCluster,
    BigQueryDataset,
    MemorystoreInstance,
    AppEngineVersion,
    PubSubTopic,
    PubSubSubscription,
    SpannerInstance,
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceType::VirtualMachine => write!(f, "Virtual Machine"),
            ResourceType::Disk => write!(f, "Disk"),
            ResourceType::Snapshot => write!(f, "Snapshot"),
            ResourceType::LoadBalancer => write!(f, "Load Balancer"),
            ResourceType::ElasticIp => write!(f, "Elastic IP"),
            ResourceType::SecurityGroup => write!(f, "Security Group"),
            ResourceType::ServerlessFunction => write!(f, "Serverless Function"),
            ResourceType::StorageBucket => write!(f, "Storage Bucket"),
            ResourceType::MachineImage => write!(f, "Machine Image"),
            ResourceType::CloudSqlInstance => write!(f, "Cloud SQL"),
            ResourceType::CloudRunService => write!(f, "Cloud Run"),
            ResourceType::Network => write!(f, "Network"),
            ResourceType::NatGateway => write!(f, "NAT Gateway"),
            ResourceType::VpnTunnel => write!(f, "VPN Tunnel"),
            ResourceType::ArtifactRegistryRepo => write!(f, "Artifact Registry"),
            ResourceType::DataprocCluster => write!(f, "Dataproc Cluster"),
            ResourceType::SecretManagerSecret => write!(f, "Secret Manager Secret"),
            ResourceType::LogSink => write!(f, "Log Sink"),
            ResourceType::GkeCluster => write!(f, "GKE Cluster"),
            ResourceType::BigQueryDataset => write!(f, "BigQuery Dataset"),
            ResourceType::MemorystoreInstance => write!(f, "Memorystore Instance"),
            ResourceType::AppEngineVersion => write!(f, "App Engine Version"),
            ResourceType::PubSubTopic => write!(f, "Pub/Sub Topic"),
            ResourceType::PubSubSubscription => write!(f, "Pub/Sub Subscription"),
            ResourceType::SpannerInstance => write!(f, "Spanner Instance"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudResource {
    pub id: String,
    pub name: String,
    pub resource_type: ResourceType,
    pub provider: ProviderKind,
    pub region: String,
    pub account_id: String,
    pub status: String,
    pub created_at: Option<String>,
    pub last_used: Option<String>,
    pub tags: HashMap<String, String>,
    pub metadata: serde_json::Value,
    pub monthly_cost: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub account_id: String,
    pub resource_type: String,
    pub found: usize,
    pub status: ScanStepStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ScanStepStatus {
    Scanning,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudAccount {
    pub id: String,
    pub provider: ProviderKind,
    pub display_name: String,
    pub project_id: Option<String>,
    pub config: serde_json::Value,
}

#[async_trait]
pub trait CloudProvider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    fn account_id(&self) -> &str;
    async fn validate_credentials(&self) -> anyhow::Result<String>;
    async fn list_regions(&self) -> anyhow::Result<Vec<String>>;
    async fn scan_resource_type(
        &self,
        region: &str,
        resource_type: &ResourceType,
        progress_tx: tokio::sync::mpsc::Sender<ScanProgress>,
    ) -> anyhow::Result<Vec<CloudResource>>;
    fn supported_resource_types(&self) -> Vec<ResourceType>;
}
