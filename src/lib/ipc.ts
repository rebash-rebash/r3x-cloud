import { invoke } from "@tauri-apps/api/core";
import type {
  CloudAccount,
  CloudResource,
  ProviderKind,
  ScanResult,
  DetectionRule,
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
