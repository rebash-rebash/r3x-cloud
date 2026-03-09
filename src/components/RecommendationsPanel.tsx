import { For, Show } from "solid-js";
import { analysis, analyzing, analysisError, runAnalysis } from "../stores/analysis";
import { activeAccount } from "../stores/cloud";
import { formatCurrency } from "../lib/format";

function severityColor(severity: string): string {
  switch (severity) {
    case "critical": return "var(--color-danger)";
    case "high": return "#f0883e";
    case "medium": return "var(--color-warning)";
    case "low": return "var(--color-muted)";
    default: return "var(--color-muted)";
  }
}

export default function RecommendationsPanel() {
  const handleAnalyze = () => {
    const acc = activeAccount();
    if (acc) runAnalysis(acc.id);
  };

  const data = analysis;

  return (
    <div>
      <div style={{ display: "flex", "align-items": "center", gap: "12px", "margin-bottom": "16px" }}>
        <h2 style={{ "font-size": "18px" }}>Recommendations</h2>
        <button
          class="btn btn-primary btn-sm"
          onClick={handleAnalyze}
          disabled={analyzing() || !activeAccount()}
        >
          {analyzing() ? "Analyzing..." : "Run Analysis"}
        </button>
      </div>

      <Show when={analysisError()}>
        <div class="card" style={{ "border-color": "var(--color-danger)", "margin-bottom": "16px" }}>
          <div class="card-title" style={{ color: "var(--color-danger)" }}>Error</div>
          <p style={{ "font-size": "12px" }}>{analysisError()}</p>
        </div>
      </Show>

      <Show when={data()}>
        {/* Summary cards */}
        <div class="dashboard-grid" style={{ "margin-bottom": "20px" }}>
          <div class="card">
            <div class="card-title">Potential Savings</div>
            <div class="card-value" style={{ color: "var(--color-success)" }}>
              {formatCurrency(data()!.total_monthly_savings)}/mo
            </div>
          </div>
          <div class="card">
            <div class="card-title">Total Findings</div>
            <div class="card-value">{data()!.total_findings}</div>
          </div>
          <div class="card">
            <div class="card-title">Critical / High</div>
            <div class="card-value" style={{ color: "var(--color-danger)" }}>
              {data()!.critical_count + data()!.high_count}
            </div>
          </div>
          <div class="card">
            <div class="card-title">Medium / Low</div>
            <div class="card-value" style={{ color: "var(--color-warning)" }}>
              {data()!.medium_count + data()!.low_count}
            </div>
          </div>
        </div>

        {/* Findings list */}
        <Show
          when={data()!.findings.length > 0}
          fallback={
            <div class="empty-state">
              <div class="empty-state-title">No issues found</div>
              <p>Your infrastructure looks clean.</p>
            </div>
          }
        >
          <div style={{ display: "flex", "flex-direction": "column", gap: "8px" }}>
            <For each={data()!.findings}>
              {(finding) => (
                <div class="card" style={{ padding: "12px" }}>
                  <div style={{ display: "flex", "align-items": "flex-start", gap: "12px" }}>
                    {/* Severity badge */}
                    <span
                      class="status-badge"
                      style={{
                        color: severityColor(finding.severity),
                        background: `${severityColor(finding.severity)}20`,
                        "min-width": "60px",
                        "text-align": "center",
                        "text-transform": "uppercase",
                        "font-size": "10px",
                        "font-weight": "700",
                        "margin-top": "2px",
                      }}
                    >
                      {finding.severity}
                    </span>

                    <div style={{ flex: "1" }}>
                      <div style={{ "font-weight": "600", "font-size": "13px", "margin-bottom": "4px" }}>
                        {finding.rule_name}
                      </div>
                      <div style={{ "font-size": "12px", color: "var(--text-secondary)", "margin-bottom": "6px" }}>
                        {finding.description}
                      </div>
                      <div style={{ "font-size": "11px", color: "var(--text-muted)" }}>
                        {finding.recommendation}
                      </div>
                      <div style={{ "font-size": "11px", color: "var(--text-muted)", "margin-top": "4px" }}>
                        Region: {finding.region}
                      </div>
                    </div>

                    {/* Savings */}
                    <Show when={finding.estimated_monthly_savings > 0}>
                      <div style={{
                        "text-align": "right",
                        "min-width": "80px",
                      }}>
                        <div style={{ "font-size": "14px", "font-weight": "700", color: "var(--color-success)" }}>
                          {formatCurrency(finding.estimated_monthly_savings)}
                        </div>
                        <div style={{ "font-size": "10px", color: "var(--text-muted)" }}>/month</div>
                      </div>
                    </Show>
                  </div>
                </div>
              )}
            </For>
          </div>
        </Show>
      </Show>

      <Show when={!data() && !analyzing()}>
        <div class="empty-state">
          <div class="empty-state-title">No analysis data</div>
          <p>Run a scan first, then click "Run Analysis" to detect unused resources.</p>
        </div>
      </Show>
    </div>
  );
}
