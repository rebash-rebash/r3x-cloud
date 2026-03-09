import { Show, For, createMemo } from "solid-js";
import { resources } from "../stores/scan";
import { activeAccount } from "../stores/cloud";
import { analysis } from "../stores/analysis";
import { formatCurrency, formatResourceType } from "../lib/format";
import CostTrend from "./CostTrend";

// SVG donut chart
function DonutChart(props: {
  segments: { label: string; value: number; color: string }[];
  total: number;
  centerLabel: string;
  centerValue: string;
}) {
  const R = 54;
  const SW = 16;
  const C = 70;
  const circ = 2 * Math.PI * R;

  const arcs = () => {
    let off = 0;
    return props.segments.map((seg) => {
      const pct = props.total > 0 ? seg.value / props.total : 0;
      const len = pct * circ;
      const dashOff = -off;
      off += len;
      return { ...seg, len, dashOff };
    });
  };

  return (
    <div style={{ display: "flex", "align-items": "center", gap: "16px" }}>
      <svg width="140" height="140" viewBox="0 0 140 140">
        <circle cx={C} cy={C} r={R} fill="none" stroke="var(--bg-tertiary)" stroke-width={SW} />
        <For each={arcs()}>
          {(a) => (
            <circle cx={C} cy={C} r={R} fill="none" stroke={a.color} stroke-width={SW}
              stroke-dasharray={`${a.len} ${circ - a.len}`} stroke-dashoffset={a.dashOff}
              transform={`rotate(-90 ${C} ${C})`} />
          )}
        </For>
        <text x={C} y={C - 6} text-anchor="middle" fill="var(--text-primary)" font-size="18" font-weight="700">{props.centerValue}</text>
        <text x={C} y={C + 12} text-anchor="middle" fill="var(--text-muted)" font-size="10">{props.centerLabel}</text>
      </svg>
      <div style={{ display: "flex", "flex-direction": "column", gap: "3px" }}>
        <For each={arcs()}>
          {(a) => (
            <Show when={a.value > 0}>
              <div style={{ display: "flex", "align-items": "center", gap: "6px", "font-size": "11px" }}>
                <span style={{ width: "8px", height: "8px", "border-radius": "50%", background: a.color, "flex-shrink": "0" }} />
                <span style={{ color: "var(--text-secondary)" }}>{a.label}</span>
                <span style={{ color: "var(--text-muted)", "margin-left": "auto" }}>{a.value}</span>
              </div>
            </Show>
          )}
        </For>
      </div>
    </div>
  );
}

// Horizontal bar chart
function BarChart(props: { bars: { label: string; value: number }[]; formatValue?: (n: number) => string }) {
  const maxVal = () => Math.max(...props.bars.map((b) => b.value), 1);
  const fmt = props.formatValue || ((n: number) => String(n));

  return (
    <div style={{ display: "flex", "flex-direction": "column", gap: "4px" }}>
      <For each={props.bars}>
        {(bar) => (
          <div style={{ display: "flex", "align-items": "center", gap: "8px", "font-size": "11px" }}>
            <span style={{ width: "90px", "text-align": "right", color: "var(--text-muted)", overflow: "hidden", "text-overflow": "ellipsis", "white-space": "nowrap", "flex-shrink": "0" }} title={bar.label}>
              {bar.label}
            </span>
            <div style={{ flex: "1", height: "14px", background: "var(--bg-tertiary)", "border-radius": "3px", overflow: "hidden" }}>
              <div style={{
                height: "100%", width: `${(bar.value / maxVal()) * 100}%`,
                background: "var(--color-accent)", "border-radius": "3px",
                "min-width": bar.value > 0 ? "2px" : "0",
              }} />
            </div>
            <span style={{ width: "60px", color: "var(--text-secondary)", "font-weight": "500" }}>{fmt(bar.value)}</span>
          </div>
        )}
      </For>
    </div>
  );
}

const TYPE_COLORS: Record<string, string> = {
  virtual_machine: "#58a6ff", disk: "#d29922", snapshot: "#8b949e",
  elastic_ip: "#3fb950", load_balancer: "#bc8cff", security_group: "#f0883e",
  machine_image: "#6e7681", storage_bucket: "#79c0ff", serverless_function: "#d2a8ff",
  cloud_sql_instance: "#ff7b72", cloud_run_service: "#7ee787", network: "#a5d6ff",
  gke_cluster: "#f778ba", big_query_dataset: "#ffa657", pub_sub_topic: "#a371f7",
  pub_sub_subscription: "#8957e5", spanner_instance: "#e3b341", memorystore_instance: "#f47067",
  app_engine_version: "#57ab5a", nat_gateway: "#539bf5", vpn_tunnel: "#986ee2",
  artifact_registry_repo: "#c69026", dataproc_cluster: "#66d9ef", secret_manager_secret: "#e06c75",
  log_sink: "#56d4dd",
};

export default function Dashboard() {
  const totalResources = () => resources().length;
  const totalMonthlyCost = createMemo(() => resources().reduce((sum, r) => sum + (r.monthly_cost || 0), 0));
  const stoppedVMs = createMemo(() => resources().filter((r) => r.resource_type === "virtual_machine" && r.status.toUpperCase() === "TERMINATED").length);
  const runningVMs = createMemo(() => resources().filter((r) => r.resource_type === "virtual_machine" && r.status.toUpperCase() === "RUNNING").length);
  const regionCount = createMemo(() => new Set(resources().map((r) => r.region)).size);
  const data = analysis;

  // Cost by type
  const costByType = createMemo(() => {
    const map = new Map<string, number>();
    for (const r of resources()) map.set(r.resource_type, (map.get(r.resource_type) ?? 0) + (r.monthly_cost ?? 0));
    return [...map.entries()].sort((a, b) => b[1] - a[1]).map(([t, c]) => ({ label: formatResourceType(t), value: c, color: TYPE_COLORS[t] || "#8b949e" }));
  });

  // Count by type (donut)
  const countByType = createMemo(() => {
    const map = new Map<string, number>();
    for (const r of resources()) map.set(r.resource_type, (map.get(r.resource_type) ?? 0) + 1);
    return [...map.entries()].sort((a, b) => b[1] - a[1]).map(([t, c]) => ({ label: formatResourceType(t), value: c, color: TYPE_COLORS[t] || "#8b949e" }));
  });

  // Cost by region
  const costByRegion = createMemo(() => {
    const map = new Map<string, number>();
    for (const r of resources()) map.set(r.region || "unknown", (map.get(r.region || "unknown") ?? 0) + (r.monthly_cost ?? 0));
    return [...map.entries()].sort((a, b) => b[1] - a[1]).slice(0, 8).map(([r, c]) => ({ label: r, value: c }));
  });

  // Top 10 expensive
  const topExpensive = createMemo(() =>
    [...resources()].filter((r) => r.monthly_cost && r.monthly_cost > 0)
      .sort((a, b) => (b.monthly_cost ?? 0) - (a.monthly_cost ?? 0)).slice(0, 10)
  );

  // Tag compliance
  const untaggedCount = createMemo(() =>
    resources().filter((r) => r.resource_type !== "security_group" && Object.keys(r.tags).length === 0).length
  );
  const tagCompliancePct = createMemo(() => {
    const taggable = resources().filter((r) => r.resource_type !== "security_group").length;
    if (taggable === 0) return 100;
    return Math.round(((taggable - untaggedCount()) / taggable) * 100);
  });

  // Severity donut
  const severitySegments = createMemo(() => {
    const d = data();
    if (!d) return [];
    return [
      { label: "Critical", value: d.critical_count, color: "#f85149" },
      { label: "High", value: d.high_count, color: "#f0883e" },
      { label: "Medium", value: d.medium_count, color: "#d29922" },
      { label: "Low", value: d.low_count, color: "#8b949e" },
    ];
  });

  return (
    <div>
      <h2 style={{ "margin-bottom": "16px", "font-size": "18px" }}>Dashboard</h2>

      <Show when={activeAccount()} fallback={
        <div class="empty-state">
          <div class="empty-state-title">No account configured</div>
          <p>Add a cloud account to get started.</p>
        </div>
      }>
        <Show when={totalResources() > 0}>
          {/* Key metrics */}
          <div class="dashboard-grid">
            <div class="card">
              <div class="card-title">Total Resources</div>
              <div class="card-value">{totalResources()}</div>
            </div>
            <div class="card">
              <div class="card-title">Monthly Cost</div>
              <div class="card-value">{formatCurrency(totalMonthlyCost())}</div>
            </div>
            <div class="card">
              <div class="card-title">Potential Savings</div>
              <div class="card-value" style={{ color: data()?.total_monthly_savings ? "var(--color-success)" : undefined }}>
                {data() ? formatCurrency(data()!.total_monthly_savings) : "-"}
              </div>
            </div>
            <div class="card">
              <div class="card-title">Findings</div>
              <div class="card-value" style={{ color: data()?.total_findings ? "var(--color-danger)" : undefined }}>
                {data()?.total_findings ?? 0}
              </div>
            </div>
            <div class="card">
              <div class="card-title">VMs</div>
              <div style={{ display: "flex", gap: "12px", "align-items": "baseline" }}>
                <span class="card-value" style={{ color: "var(--color-success)" }}>{runningVMs()}</span>
                <Show when={stoppedVMs() > 0}>
                  <span style={{ "font-size": "14px", color: "var(--color-danger)" }}>{stoppedVMs()} stopped</span>
                </Show>
              </div>
            </div>
            <div class="card">
              <div class="card-title">Tag Compliance</div>
              <div class="card-value" style={{
                color: tagCompliancePct() >= 80 ? "var(--color-success)"
                  : tagCompliancePct() >= 50 ? "var(--color-warning)" : "var(--color-danger)",
              }}>
                {tagCompliancePct()}%
              </div>
              <div style={{ "font-size": "11px", color: "var(--text-muted)", "margin-top": "2px" }}>
                {untaggedCount()} untagged
              </div>
            </div>
          </div>

          {/* Charts row */}
          <div style={{ display: "grid", "grid-template-columns": "1fr 1fr", gap: "16px", "margin-bottom": "16px" }}>
            <div class="card">
              <div class="card-title">Resources by Type</div>
              <div style={{ "margin-top": "8px" }}>
                <DonutChart segments={countByType()} total={totalResources()} centerValue={String(totalResources())} centerLabel="total" />
              </div>
            </div>

            <Show when={data() && data()!.total_findings > 0} fallback={
              <div class="card">
                <div class="card-title">Summary</div>
                <div style={{ "margin-top": "12px", "font-size": "12px", color: "var(--text-secondary)", display: "flex", "flex-direction": "column", gap: "8px" }}>
                  <div>{regionCount()} regions scanned</div>
                  <div>{totalResources() - untaggedCount()} tagged resources</div>
                  <div>{untaggedCount()} resources need labels</div>
                </div>
              </div>
            }>
              <div class="card">
                <div class="card-title">Findings by Severity</div>
                <div style={{ "margin-top": "8px" }}>
                  <DonutChart segments={severitySegments()} total={data()!.total_findings} centerValue={String(data()!.total_findings)} centerLabel="findings" />
                </div>
              </div>
            </Show>
          </div>

          {/* Cost Trend */}
          <CostTrend />

          {/* Bar charts row */}
          <div style={{ display: "grid", "grid-template-columns": "1fr 1fr", gap: "16px", "margin-bottom": "16px" }}>
            <Show when={costByRegion().length > 0}>
              <div class="card">
                <div class="card-title">Cost by Region</div>
                <div style={{ "margin-top": "8px" }}>
                  <BarChart bars={costByRegion()} formatValue={formatCurrency} />
                </div>
              </div>
            </Show>
            <Show when={costByType().length > 0}>
              <div class="card">
                <div class="card-title">Cost by Type</div>
                <div style={{ "margin-top": "8px" }}>
                  <BarChart bars={costByType().map((t) => ({ label: t.label, value: t.value }))} formatValue={formatCurrency} />
                </div>
              </div>
            </Show>
          </div>

          {/* Top expensive */}
          <Show when={topExpensive().length > 0}>
            <div class="card" style={{ "margin-bottom": "16px" }}>
              <div class="card-title">Top 10 Most Expensive Resources</div>
              <div style={{ "margin-top": "8px" }}>
                <table class="resource-table" style={{ "font-size": "11px" }}>
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
                    <For each={topExpensive()}>
                      {(r) => (
                        <tr>
                          <td title={r.name}>{r.name}</td>
                          <td>{formatResourceType(r.resource_type)}</td>
                          <td>{r.region}</td>
                          <td>{r.status}</td>
                          <td style={{ "text-align": "right", "font-weight": "600" }}>{formatCurrency(r.monthly_cost)}</td>
                        </tr>
                      )}
                    </For>
                  </tbody>
                </table>
              </div>
            </div>
          </Show>
        </Show>

        <Show when={totalResources() === 0}>
          <div class="empty-state">
            <div class="empty-state-title">No scan data</div>
            <p>Click <kbd>Scan</kbd> or press <kbd>s</kbd> to scan your cloud resources.</p>
          </div>
        </Show>
      </Show>
    </div>
  );
}
