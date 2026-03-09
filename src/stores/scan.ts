import { createSignal } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import type { CloudResource, ScanProgress, ScanResult } from "../lib/types";
import * as ipc from "../lib/ipc";
import { runAnalysis as runAnalysisStore } from "./analysis";

const [scanning, setScanning] = createSignal(false);
const [scanProgress, setScanProgress] = createSignal<ScanProgress[]>([]);
const [resources, setResources] = createSignal<CloudResource[]>([]);
const [lastScanResult, setLastScanResult] = createSignal<ScanResult | null>(null);
const [scanError, setScanError] = createSignal<string | null>(null);

export { scanning, scanProgress, resources, lastScanResult, scanError };

// Listen for scan progress events from backend
let progressListenerSetup = false;
export function setupScanListeners() {
  if (progressListenerSetup) return;
  progressListenerSetup = true;

  listen<ScanProgress>("scan-progress", (event) => {
    setScanProgress((prev) => {
      const existing = prev.findIndex(
        (p) =>
          p.account_id === event.payload.account_id &&
          p.resource_type === event.payload.resource_type,
      );
      if (existing >= 0) {
        const updated = [...prev];
        updated[existing] = event.payload;
        return updated;
      }
      return [...prev, event.payload];
    });
  });
}

export async function startScan(accountId: string) {
  setScanning(true);
  setScanProgress([]);
  setScanError(null);
  setResources([]);

  try {
    const result = await ipc.startScan(accountId);
    setLastScanResult(result);

    // Load the resources from the completed scan
    const scanResources = await ipc.getScanResources(result.scan_id);
    setResources(scanResources);

    // Auto-run analysis after scan completes
    runAnalysisStore(accountId).catch(() => {});
  } catch (e) {
    setScanError(String(e));
  } finally {
    setScanning(false);
  }
}

export async function loadLatestResources(accountId: string) {
  try {
    const result = await ipc.getLatestResources(accountId);
    setResources(result);
  } catch (e) {
    setScanError(String(e));
  }
}
