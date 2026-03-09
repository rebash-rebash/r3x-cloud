import { createSignal, createMemo, Show, For, onMount } from "solid-js";
import { activeAccount } from "../stores/cloud";
import { formatCurrency } from "../lib/format";
import type { CostTrendPoint } from "../lib/types";
import * as ipc from "../lib/ipc";

export default function CostTrend() {
  const [trend, setTrend] = createSignal<CostTrendPoint[]>([]);
  const [loading, setLoading] = createSignal(false);

  const loadTrend = async () => {
    const acc = activeAccount();
    if (!acc) return;
    setLoading(true);
    try {
      const data = await ipc.getCostTrend(acc.id);
      setTrend(data);
    } catch { /* ignore */ }
    finally { setLoading(false); }
  };

  onMount(loadTrend);

  // Chart dimensions
  const W = 560;
  const H = 200;
  const PAD = { top: 20, right: 16, bottom: 30, left: 60 };
  const chartW = W - PAD.left - PAD.right;
  const chartH = H - PAD.top - PAD.bottom;

  const maxCost = createMemo(() => Math.max(...trend().map((p) => p.total_monthly_cost), 1));

  const points = createMemo(() =>
    trend().map((p, i) => {
      const x = PAD.left + (trend().length > 1 ? (i / (trend().length - 1)) * chartW : chartW / 2);
      const y = PAD.top + chartH - (p.total_monthly_cost / maxCost()) * chartH;
      return { x, y, ...p };
    })
  );

  const linePath = createMemo(() => {
    const pts = points();
    if (pts.length === 0) return "";
    return pts.map((p, i) => `${i === 0 ? "M" : "L"} ${p.x} ${p.y}`).join(" ");
  });

  const areaPath = createMemo(() => {
    const pts = points();
    if (pts.length === 0) return "";
    const bottom = PAD.top + chartH;
    return `${linePath()} L ${pts[pts.length - 1].x} ${bottom} L ${pts[0].x} ${bottom} Z`;
  });

  const yTicks = createMemo(() => {
    const max = maxCost();
    const ticks: number[] = [];
    const step = max > 0 ? Math.pow(10, Math.floor(Math.log10(max))) : 1;
    for (let v = 0; v <= max; v += step) ticks.push(v);
    if (ticks[ticks.length - 1] < max) ticks.push(max);
    // Limit to ~5 ticks
    if (ticks.length > 6) {
      const newStep = step * 2;
      ticks.length = 0;
      for (let v = 0; v <= max; v += newStep) ticks.push(v);
      if (ticks[ticks.length - 1] < max) ticks.push(max);
    }
    return ticks;
  });

  const formatDate = (iso: string) => {
    try {
      const d = new Date(iso);
      return `${d.getMonth() + 1}/${d.getDate()}`;
    } catch { return iso; }
  };

  const costDelta = createMemo(() => {
    const t = trend();
    if (t.length < 2) return null;
    const first = t[0].total_monthly_cost;
    const last = t[t.length - 1].total_monthly_cost;
    return { amount: last - first, pct: first > 0 ? ((last - first) / first) * 100 : 0 };
  });

  return (
    <div class="card" style={{ "margin-bottom": "16px" }}>
      <div style={{ display: "flex", "align-items": "center", "justify-content": "space-between" }}>
        <div class="card-title" style={{ "margin-bottom": "0" }}>Cost Trend</div>
        <div style={{ display: "flex", "align-items": "center", gap: "8px" }}>
          <Show when={costDelta()}>
            <span style={{
              "font-size": "12px",
              "font-weight": "600",
              color: costDelta()!.amount <= 0 ? "var(--color-success)" : "var(--color-danger)",
            }}>
              {costDelta()!.amount <= 0 ? "" : "+"}{formatCurrency(costDelta()!.amount)}
              {" "}({costDelta()!.pct >= 0 ? "+" : ""}{costDelta()!.pct.toFixed(1)}%)
            </span>
          </Show>
          <button class="btn btn-sm" onClick={loadTrend} disabled={loading()}>
            {loading() ? "..." : "Refresh"}
          </button>
        </div>
      </div>

      <Show when={trend().length >= 2} fallback={
        <div style={{ "font-size": "12px", color: "var(--text-muted)", "margin-top": "12px" }}>
          {loading() ? "Loading trend data..." : "Need at least 2 scans for trend chart. Run more scans to see cost over time."}
        </div>
      }>
        <div style={{ "margin-top": "12px", overflow: "hidden" }}>
          <svg width={W} height={H} viewBox={`0 0 ${W} ${H}`} style={{ width: "100%", height: "auto" }}>
            {/* Y-axis grid lines & labels */}
            <For each={yTicks()}>
              {(tick) => {
                const y = PAD.top + chartH - (tick / maxCost()) * chartH;
                return (
                  <>
                    <line x1={PAD.left} y1={y} x2={PAD.left + chartW} y2={y} stroke="var(--border-secondary)" stroke-width="1" />
                    <text x={PAD.left - 8} y={y + 4} text-anchor="end" fill="var(--text-muted)" font-size="9">
                      {formatCurrency(tick)}
                    </text>
                  </>
                );
              }}
            </For>

            {/* Area fill */}
            <path d={areaPath()} fill="var(--color-accent)" opacity="0.15" />

            {/* Line */}
            <path d={linePath()} fill="none" stroke="var(--color-accent)" stroke-width="2" stroke-linejoin="round" />

            {/* Data points */}
            <For each={points()}>
              {(p) => (
                <circle cx={p.x} cy={p.y} r="3" fill="var(--color-accent)" stroke="var(--bg-primary)" stroke-width="1.5" />
              )}
            </For>

            {/* X-axis labels */}
            <For each={points()}>
              {(p, i) => {
                // Show every nth label to avoid overlap
                const step = Math.max(1, Math.floor(points().length / 6));
                if (i() % step !== 0 && i() !== points().length - 1) return null;
                return (
                  <text x={p.x} y={PAD.top + chartH + 16} text-anchor="middle" fill="var(--text-muted)" font-size="9">
                    {formatDate(p.completed_at)}
                  </text>
                );
              }}
            </For>
          </svg>
        </div>

        {/* Summary table */}
        <div style={{ display: "flex", gap: "16px", "margin-top": "8px", "font-size": "11px", color: "var(--text-muted)" }}>
          <span>{trend().length} scans</span>
          <span>Latest: {formatCurrency(trend()[trend().length - 1]?.total_monthly_cost ?? 0)}/mo</span>
          <span>{trend()[trend().length - 1]?.resource_count ?? 0} resources</span>
        </div>
      </Show>
    </div>
  );
}
