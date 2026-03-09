import { createSignal, createMemo, For, Show, onMount } from "solid-js";
import { activeAccount } from "../stores/cloud";
import * as ipc from "../lib/ipc";
import type { ScanRecord, CloudResource } from "../lib/types";
import { formatCurrency, formatResourceType, formatDate } from "../lib/format";

interface DiffResult {
  added: CloudResource[];
  removed: CloudResource[];
  changed: { resource: CloudResource; oldCost: number | null; newCost: number | null; oldStatus: string; newStatus: string }[];
  costDelta: number;
}

export default function ScanDiff() {
  const [scans, setScans] = createSignal<ScanRecord[]>([]);
  const [scanA, setScanA] = createSignal<string>("");
  const [scanB, setScanB] = createSignal<string>("");
  const [diff, setDiff] = createSignal<DiffResult | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [tab, setTab] = createSignal<"added" | "removed" | "changed">("added");

  onMount(async () => {
    const acc = activeAccount();
    if (!acc) return;
    try {
      const s = await ipc.listScans(acc.id);
      const completed = s.filter((sc) => sc.status === "completed");
      setScans(completed);
      if (completed.length >= 2) {
        setScanB(completed[0].id); // newest
        setScanA(completed[1].id); // previous
      }
    } catch { /* ignore */ }
  });

  const computeDiff = async () => {
    if (!scanA() || !scanB()) return;
    setLoading(true);
    try {
      const [resA, resB] = await Promise.all([
        ipc.getScanResources(scanA()),
        ipc.getScanResources(scanB()),
      ]);

      const mapA = new Map(resA.map((r) => [r.id, r]));
      const mapB = new Map(resB.map((r) => [r.id, r]));

      const added: CloudResource[] = [];
      const removed: CloudResource[] = [];
      const changed: DiffResult["changed"] = [];

      // Find added and changed
      for (const [id, resNew] of mapB) {
        const resOld = mapA.get(id);
        if (!resOld) {
          added.push(resNew);
        } else if (resOld.status !== resNew.status || resOld.monthly_cost !== resNew.monthly_cost) {
          changed.push({
            resource: resNew,
            oldCost: resOld.monthly_cost,
            newCost: resNew.monthly_cost,
            oldStatus: resOld.status,
            newStatus: resNew.status,
          });
        }
      }

      // Find removed
      for (const [id, resOld] of mapA) {
        if (!mapB.has(id)) {
          removed.push(resOld);
        }
      }

      const costA = resA.reduce((s, r) => s + (r.monthly_cost || 0), 0);
      const costB = resB.reduce((s, r) => s + (r.monthly_cost || 0), 0);

      setDiff({
        added,
        removed,
        changed,
        costDelta: costB - costA,
      });
    } catch { /* ignore */ } finally {
      setLoading(false);
    }
  };

  // Auto-compute when selections change
  createMemo(() => {
    if (scanA() && scanB() && scanA() !== scanB()) computeDiff();
  });

  const scanLabel = (id: string) => {
    const s = scans().find((sc) => sc.id === id);
    if (!s) return id;
    return `${formatDate(s.started_at)} (${s.resource_count} resources)`;
  };

  return (
    <div>
      <h2 style={{ "font-size": "18px", "margin-bottom": "16px" }}>Scan Diff</h2>

      <Show when={scans().length < 2} fallback={
        <>
          {/* Scan selectors */}
          <div class="card" style={{ "margin-bottom": "16px" }}>
            <div style={{ display: "flex", gap: "16px", "align-items": "center" }}>
              <div class="form-group" style={{ flex: "1" }}>
                <label>Baseline (older)</label>
                <select
                  class="project-select"
                  style={{ width: "100%" }}
                  value={scanA()}
                  onChange={(e) => setScanA(e.currentTarget.value)}
                >
                  <For each={scans()}>
                    {(s) => <option value={s.id}>{scanLabel(s.id)}</option>}
                  </For>
                </select>
              </div>
              <span style={{ color: "var(--text-muted)", "font-size": "18px", "margin-top": "16px" }}>→</span>
              <div class="form-group" style={{ flex: "1" }}>
                <label>Current (newer)</label>
                <select
                  class="project-select"
                  style={{ width: "100%" }}
                  value={scanB()}
                  onChange={(e) => setScanB(e.currentTarget.value)}
                >
                  <For each={scans()}>
                    {(s) => <option value={s.id}>{scanLabel(s.id)}</option>}
                  </For>
                </select>
              </div>
            </div>
          </div>

          <Show when={loading()}>
            <div style={{ color: "var(--text-muted)", "font-size": "13px", padding: "16px" }}>Comparing scans...</div>
          </Show>

          <Show when={diff() && !loading()}>
            {/* Summary cards */}
            <div class="dashboard-grid" style={{ "margin-bottom": "16px" }}>
              <div class="card" onClick={() => setTab("added")} style={{ cursor: "pointer", border: tab() === "added" ? "1px solid var(--color-success)" : undefined }}>
                <div class="card-title">Added</div>
                <div class="card-value" style={{ color: "var(--color-success)" }}>+{diff()!.added.length}</div>
              </div>
              <div class="card" onClick={() => setTab("removed")} style={{ cursor: "pointer", border: tab() === "removed" ? "1px solid var(--color-danger)" : undefined }}>
                <div class="card-title">Removed</div>
                <div class="card-value" style={{ color: "var(--color-danger)" }}>-{diff()!.removed.length}</div>
              </div>
              <div class="card" onClick={() => setTab("changed")} style={{ cursor: "pointer", border: tab() === "changed" ? "1px solid var(--color-warning)" : undefined }}>
                <div class="card-title">Changed</div>
                <div class="card-value" style={{ color: "var(--color-warning)" }}>{diff()!.changed.length}</div>
              </div>
              <div class="card">
                <div class="card-title">Cost Delta</div>
                <div class="card-value" style={{ color: diff()!.costDelta > 0 ? "var(--color-danger)" : diff()!.costDelta < 0 ? "var(--color-success)" : "var(--text-primary)" }}>
                  {diff()!.costDelta > 0 ? "+" : ""}{formatCurrency(diff()!.costDelta)}/mo
                </div>
              </div>
            </div>

            {/* Detail table */}
            <div class="card">
              <Show when={tab() === "added"}>
                <div class="card-title">New Resources ({diff()!.added.length})</div>
                <Show when={diff()!.added.length === 0}>
                  <div style={{ "font-size": "12px", color: "var(--text-muted)", padding: "8px 0" }}>No new resources</div>
                </Show>
                <Show when={diff()!.added.length > 0}>
                  <table class="resource-table" style={{ "font-size": "11px", "margin-top": "8px" }}>
                    <thead>
                      <tr>
                        <th>Name</th>
                        <th>Type</th>
                        <th>Region</th>
                        <th>Status</th>
                        <th style={{ "text-align": "right" }}>Cost/mo</th>
                      </tr>
                    </thead>
                    <tbody>
                      <For each={diff()!.added}>
                        {(r) => (
                          <tr>
                            <td style={{ color: "var(--color-success)" }}>{r.name}</td>
                            <td>{formatResourceType(r.resource_type)}</td>
                            <td>{r.region}</td>
                            <td>{r.status}</td>
                            <td style={{ "text-align": "right" }}>{formatCurrency(r.monthly_cost)}</td>
                          </tr>
                        )}
                      </For>
                    </tbody>
                  </table>
                </Show>
              </Show>

              <Show when={tab() === "removed"}>
                <div class="card-title">Removed Resources ({diff()!.removed.length})</div>
                <Show when={diff()!.removed.length === 0}>
                  <div style={{ "font-size": "12px", color: "var(--text-muted)", padding: "8px 0" }}>No removed resources</div>
                </Show>
                <Show when={diff()!.removed.length > 0}>
                  <table class="resource-table" style={{ "font-size": "11px", "margin-top": "8px" }}>
                    <thead>
                      <tr>
                        <th>Name</th>
                        <th>Type</th>
                        <th>Region</th>
                        <th>Status</th>
                        <th style={{ "text-align": "right" }}>Cost/mo</th>
                      </tr>
                    </thead>
                    <tbody>
                      <For each={diff()!.removed}>
                        {(r) => (
                          <tr>
                            <td style={{ color: "var(--color-danger)", "text-decoration": "line-through" }}>{r.name}</td>
                            <td>{formatResourceType(r.resource_type)}</td>
                            <td>{r.region}</td>
                            <td>{r.status}</td>
                            <td style={{ "text-align": "right" }}>{formatCurrency(r.monthly_cost)}</td>
                          </tr>
                        )}
                      </For>
                    </tbody>
                  </table>
                </Show>
              </Show>

              <Show when={tab() === "changed"}>
                <div class="card-title">Changed Resources ({diff()!.changed.length})</div>
                <Show when={diff()!.changed.length === 0}>
                  <div style={{ "font-size": "12px", color: "var(--text-muted)", padding: "8px 0" }}>No changes detected</div>
                </Show>
                <Show when={diff()!.changed.length > 0}>
                  <table class="resource-table" style={{ "font-size": "11px", "margin-top": "8px" }}>
                    <thead>
                      <tr>
                        <th>Name</th>
                        <th>Type</th>
                        <th>Status Change</th>
                        <th style={{ "text-align": "right" }}>Cost Change</th>
                      </tr>
                    </thead>
                    <tbody>
                      <For each={diff()!.changed}>
                        {(c) => (
                          <tr>
                            <td>{c.resource.name}</td>
                            <td>{formatResourceType(c.resource.resource_type)}</td>
                            <td>
                              <Show when={c.oldStatus !== c.newStatus} fallback={<span style={{ color: "var(--text-muted)" }}>—</span>}>
                                <span style={{ color: "var(--text-muted)" }}>{c.oldStatus}</span>
                                <span style={{ color: "var(--text-muted)", margin: "0 4px" }}>→</span>
                                <span style={{ color: "var(--color-warning)" }}>{c.newStatus}</span>
                              </Show>
                            </td>
                            <td style={{ "text-align": "right" }}>
                              <Show when={c.oldCost !== c.newCost} fallback={<span style={{ color: "var(--text-muted)" }}>—</span>}>
                                <span style={{ color: "var(--text-muted)" }}>{formatCurrency(c.oldCost)}</span>
                                <span style={{ color: "var(--text-muted)", margin: "0 4px" }}>→</span>
                                <span style={{ color: (c.newCost || 0) > (c.oldCost || 0) ? "var(--color-danger)" : "var(--color-success)" }}>
                                  {formatCurrency(c.newCost)}
                                </span>
                              </Show>
                            </td>
                          </tr>
                        )}
                      </For>
                    </tbody>
                  </table>
                </Show>
              </Show>
            </div>
          </Show>
        </>
      }>
        <div class="empty-state">
          <div class="empty-state-title">Need at least 2 scans</div>
          <p>Run multiple scans to compare changes over time.</p>
        </div>
      </Show>
    </div>
  );
}
