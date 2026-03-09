export type ProviderKind = "gcp" | "aws" | "azure";

export interface GcpProject {
  project_id: string;
  name: string;
  state: string;
}

export type ResourceType =
  | "virtual_machine"
  | "disk"
  | "snapshot"
  | "load_balancer"
  | "elastic_ip"
  | "security_group"
  | "serverless_function"
  | "storage_bucket"
  | "machine_image"
  | "cloud_sql_instance"
  | "cloud_run_service"
  | "network"
  | "gke_cluster"
  | "big_query_dataset"
  | "pub_sub_topic"
  | "pub_sub_subscription"
  | "spanner_instance"
  | "memorystore_instance"
  | "app_engine_version"
  | "nat_gateway"
  | "vpn_tunnel"
  | "artifact_registry_repo"
  | "dataproc_cluster"
  | "secret_manager_secret"
  | "log_sink";

export interface CloudAccount {
  id: string;
  provider: ProviderKind;
  display_name: string;
  project_id: string | null;
  config: Record<string, unknown>;
}

export interface CloudResource {
  id: string;
  name: string;
  resource_type: ResourceType;
  provider: ProviderKind;
  region: string;
  account_id: string;
  status: string;
  created_at: string | null;
  last_used: string | null;
  tags: Record<string, string>;
  metadata: Record<string, unknown>;
  monthly_cost: number | null;
}

export interface ScanProgress {
  account_id: string;
  resource_type: string;
  found: number;
  status: "scanning" | "completed" | "failed";
}

export interface ScanResult {
  scan_id: string;
  account_id: string;
  total_resources: number;
  status: string;
}

export interface DetectionRule {
  id: string;
  name: string;
  description: string;
  severity: "low" | "medium" | "high" | "critical";
  resource_type: string;
  enabled: boolean;
}

export interface Finding {
  id: string;
  resource_id: string;
  resource_name: string;
  resource_type: string;
  rule_id: string;
  rule_name: string;
  severity: "low" | "medium" | "high" | "critical";
  description: string;
  recommendation: string;
  estimated_monthly_savings: number;
  region: string;
}

export interface AnalysisSummary {
  total_findings: number;
  total_monthly_savings: number;
  critical_count: number;
  high_count: number;
  medium_count: number;
  low_count: number;
  findings: Finding[];
}

export interface ScanRecord {
  id: string;
  account_id: string;
  started_at: string;
  completed_at: string | null;
  status: string;
  resource_count: number;
}

export interface RuleConfigInput {
  rule_id: string;
  enabled: boolean;
}

export interface PricingEntry {
  resource_type: string;
  sku: string;
  unit: string;
  price_per_unit: number;
  region: string;
}

export interface CredentialStatus {
  provider: string;
  authenticated: boolean;
  identity: string;
  method: string;
}

export interface CostTrendPoint {
  scan_id: string;
  completed_at: string;
  total_monthly_cost: number;
  resource_count: number;
}

export type ViewKind =
  | "dashboard"
  | "resources"
  | "scan"
  | "recommendations"
  | "accounts"
  | "settings"
  | "history"
  | "map"
  | "diff";
