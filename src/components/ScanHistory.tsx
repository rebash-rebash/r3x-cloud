import { createSignal, For, Show, onMount } from "solid-js";
import type { ScanRecord } from "../lib/types";
import { activeAccount } from "../stores/cloud";
import { formatDate } from "../lib/format";
import * as ipc from "../lib/ipc";

export default function ScanHistory() {
  const [scans, setScans] = createSignal<ScanRecord[]>([]);
  const [loading, setLoading] = createSignal(false);

  const loadScans = async () => {
    const acc = activeAccount();
    if (!acc) return;
    setLoading(true);
    try {
      const result = await ipc.listScans(acc.id);
      setScans(result);
    } catch {
      setScans([]);
    } finally {
      setLoading(false);
    }
  };

  onMount(loadScans);

  const statusIcon = (status: string) => {
    if (status === "completed") return "OK";
    if (status === "failed") return "FAIL";
    return "...";
  };

  const statusColor = (status: string) => {
    if (status === "completed") return "var(--color-success)";
    if (status === "failed") return "var(--color-danger)";
    return "var(--color-warning)";
  };

  return (
    <div>
      <div style={{ display: "flex", "align-items": "center", gap: "12px", "margin-bottom": "16px" }}>
        <h2 style={{ "font-size": "18px" }}>Scan History</h2>
        <button class="btn btn-sm" onClick={loadScans} disabled={loading()}>
          {loading() ? "Loading..." : "Refresh"}
        </button>
      </div>

      <Show when={!activeAccount()}>
        <div class="empty-state">
          <div class="empty-state-title">No account selected</div>
          <p>Select an account to view scan history.</p>
        </div>
      </Show>

      <Show when={activeAccount() && scans().length === 0 && !loading()}>
        <div class="empty-state">
          <div class="empty-state-title">No scans yet</div>
          <p>Run your first scan to see history here.</p>
        </div>
      </Show>

      <Show when={scans().length > 0}>
        <div style={{ display: "flex", "flex-direction": "column", gap: "8px" }}>
          <For each={scans()}>
            {(scan) => (
              <div class="card" style={{ padding: "12px" }}>
                <div style={{ display: "flex", "align-items": "center", gap: "12px" }}>
                  <span
                    class="status-badge"
                    style={{
                      color: statusColor(scan.status),
                      background: `${statusColor(scan.status)}20`,
                      "font-size": "10px",
                      "font-weight": "700",
                    }}
                  >
                    {statusIcon(scan.status)}
                  </span>
                  <div style={{ flex: "1" }}>
                    <div style={{ "font-size": "12px", "font-weight": "600" }}>
                      {formatDate(scan.started_at)}
                    </div>
                    <div style={{ "font-size": "11px", color: "var(--text-muted)" }}>
                      {scan.resource_count} resources
                      <Show when={scan.completed_at}>
                        {" "}&middot; completed {formatDate(scan.completed_at)}
                      </Show>
                    </div>
                  </div>
                  <div style={{ "font-size": "11px", color: "var(--text-muted)", "font-family": "monospace" }}>
                    {scan.id.slice(0, 8)}
                  </div>
                </div>
              </div>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
}
