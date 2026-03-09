import { invoke } from "@tauri-apps/api/core";
import type {
  CloudAccount,
  CloudResource,
  ProviderKind,
  ScanResult,
  DetectionRule,
  AnalysisSummary,
  ScanRecord,
  GcpProject,
  RuleConfigInput,
  PricingEntry,
  CredentialStatus,
  CostTrendPoint,
} from "./types";

// Account management
export const listAccounts = () => invoke<CloudAccount[]>("list_accounts");

export const addAccount = (
  provider: ProviderKind,
  displayName: string,
  projectId: string | null,
  config: Record<string, unknown>,
) =>
  invoke<CloudAccount>("add_account", {
    provider,
    displayName,
    projectId,
    config,
  });

export const removeAccount = (id: string) =>
  invoke<void>("remove_account", { id });

export const testConnection = (
  provider: ProviderKind,
  projectId: string | null,
  config: Record<string, unknown>,
) => invoke<string>("test_connection", { provider, projectId, config });

// Scanning
export const startScan = (accountId: string) =>
  invoke<ScanResult>("start_scan", { accountId });

export const getScanResources = (scanId: string) =>
  invoke<CloudResource[]>("get_scan_resources", { scanId });

export const getLatestResources = (accountId: string) =>
  invoke<CloudResource[]>("get_latest_resources", { accountId });

// Analysis
export const listRules = () => invoke<DetectionRule[]>("list_rules");

export const runAnalysis = (accountId: string) =>
  invoke<AnalysisSummary>("run_analysis", { accountId });

// Scan history
export const listScans = (accountId: string) =>
  invoke<ScanRecord[]>("list_scans", { accountId });

// GCP projects
export const listGcpProjects = () =>
  invoke<GcpProject[]>("list_gcp_projects");

// Settings - rule configs
export const saveRuleConfigs = (configs: RuleConfigInput[]) =>
  invoke<void>("save_rule_configs", { configs });

export const getRuleConfigs = () =>
  invoke<[string, boolean][]>("get_rule_configs");

// Export
export const exportToFile = (accountId: string, format: string, exportType: string) =>
  invoke<string>("export_to_file", { accountId, format, exportType });

// Cost
export const getPricingData = () =>
  invoke<PricingEntry[]>("get_pricing_data");

// Credentials
export const checkCredentials = (provider: string) =>
  invoke<CredentialStatus>("check_credentials", { provider });

// Trend
export const getCostTrend = (accountId: string) =>
  invoke<CostTrendPoint[]>("get_cost_trend", { accountId });
