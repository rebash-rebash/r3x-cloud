export type ProviderKind = "gcp" | "aws" | "azure";

export type ResourceType =
  | "virtual_machine"
  | "disk"
  | "snapshot"
  | "load_balancer"
  | "elastic_ip"
  | "security_group"
  | "serverless_function"
  | "storage_bucket"
  | "machine_image";

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
  enabled: boolean;
}

export type ViewKind =
  | "dashboard"
  | "resources"
  | "scan"
  | "recommendations"
  | "accounts"
  | "settings";
