import { createSignal, createMemo, For, Show, onMount } from "solid-js";
import * as ipc from "../lib/ipc";
import type { DetectionRule, PricingEntry, CredentialStatus } from "../lib/types";

export default function SettingsPanel() {
  const [rules, setRules] = createSignal<DetectionRule[]>([]);
  const [ruleOverrides, setRuleOverrides] = createSignal<Record<string, boolean>>({});
  const [saved, setSaved] = createSignal(false);
  const [credStatus, setCredStatus] = createSignal<CredentialStatus | null>(null);
  const [pricing, setPricing] = createSignal<PricingEntry[]>([]);
  const [showPricing, setShowPricing] = createSignal(false);

  onMount(async () => {
    // Load rules
    try {
      const r = await ipc.listRules();
      setRules(r);
      const overrides: Record<string, boolean> = {};
      for (const rule of r) overrides[rule.id] = rule.enabled;

      // Load saved configs and merge
      try {
        const configs = await ipc.getRuleConfigs();
        for (const [id, enabled] of configs) {
          overrides[id] = enabled;
        }
      } catch { /* no saved configs yet */ }

      setRuleOverrides(overrides);
    } catch { /* ignore */ }

    // Load credential status
    try {
      const status = await ipc.checkCredentials("gcp");
      setCredStatus(status);
    } catch { /* ignore */ }

    // Load pricing
    try {
      const p = await ipc.getPricingData();
      setPricing(p);
    } catch { /* ignore */ }
  });

  const toggleRule = (id: string) => {
    setRuleOverrides((prev) => ({ ...prev, [id]: !prev[id] }));
    setSaved(false);
  };

  const saveSettings = async () => {
    try {
      const overrides = ruleOverrides();
      const configs = Object.entries(overrides).map(([id, enabled]) => ({ rule_id: id, enabled }));
      await ipc.saveRuleConfigs(configs);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch { /* ignore */ }
  };

  const severityColor = (s: string) => {
    switch (s) {
      case "critical": return "var(--color-danger)";
      case "high": return "#f0883e";
      case "medium": return "var(--color-warning)";
      case "low": return "var(--color-muted)";
      default: return "var(--text-muted)";
    }
  };

  const enabledCount = createMemo(() =>
    Object.values(ruleOverrides()).filter(Boolean).length
  );

  const groupedRules = createMemo(() => {
    const groups: Record<string, DetectionRule[]> = {};
    for (const rule of rules()) {
      const type = rule.resource_type === "*" ? "All Resources" : formatType(rule.resource_type);
      if (!groups[type]) groups[type] = [];
      groups[type].push(rule);
    }
    return Object.entries(groups).sort(([a], [b]) => {
      if (a === "All Resources") return 1;
      if (b === "All Resources") return -1;
      return a.localeCompare(b);
    });
  });

  const groupedPricing = createMemo(() => {
    const groups: Record<string, PricingEntry[]> = {};
    for (const p of pricing()) {
      const type = formatType(p.resource_type);
      if (!groups[type]) groups[type] = [];
      groups[type].push(p);
    }
    return Object.entries(groups).sort(([a], [b]) => a.localeCompare(b));
  });

  return (
    <div>
      <div style={{ display: "flex", "align-items": "center", "justify-content": "space-between", "margin-bottom": "16px" }}>
        <div>
          <h2 style={{ "font-size": "18px" }}>Settings</h2>
          <p style={{ "font-size": "12px", color: "var(--text-muted)", "margin-top": "4px" }}>
            {enabledCount()} of {rules().length} rules enabled
          </p>
        </div>
        <div style={{ display: "flex", gap: "8px", "align-items": "center" }}>
          <Show when={saved()}>
            <span style={{ "font-size": "12px", color: "var(--color-success)" }}>Saved</span>
          </Show>
          <button class="btn btn-primary" onClick={saveSettings}>Save</button>
        </div>
      </div>

      {/* Credential Status */}
      <div class="card" style={{ "margin-bottom": "16px" }}>
        <div class="card-title">Authentication</div>
        <Show when={credStatus()} fallback={
          <div style={{ "font-size": "12px", color: "var(--text-muted)" }}>Checking credentials...</div>
        }>
          <div style={{ display: "flex", "align-items": "center", gap: "12px" }}>
            <div style={{
              width: "8px", height: "8px", "border-radius": "50%",
              background: credStatus()!.authenticated ? "var(--color-success)" : "var(--color-danger)",
              "flex-shrink": "0",
            }} />
            <div style={{ flex: "1" }}>
              <div style={{ "font-size": "13px", "font-weight": "600" }}>
                {credStatus()!.authenticated ? credStatus()!.identity : "Not authenticated"}
              </div>
              <div style={{ "font-size": "11px", color: "var(--text-muted)" }}>
                {credStatus()!.provider.toUpperCase()} via {credStatus()!.method}
              </div>
            </div>
            <span style={{
              "font-size": "10px", "font-weight": "700", padding: "2px 8px", "border-radius": "4px",
              background: credStatus()!.authenticated ? "var(--color-success)" : "var(--color-danger)",
              color: "#fff", "text-transform": "uppercase",
            }}>
              {credStatus()!.authenticated ? "Active" : "Inactive"}
            </span>
          </div>
        </Show>
      </div>

      {/* Detection Rules */}
      <div style={{ display: "flex", "flex-direction": "column", gap: "16px" }}>
        <For each={groupedRules()}>
          {([group, groupRules]) => (
            <div class="card">
              <div class="card-title">{group}</div>
              <div style={{ display: "flex", "flex-direction": "column", gap: "0" }}>
                <For each={groupRules}>
                  {(rule) => (
                    <div
                      style={{
                        display: "flex",
                        "align-items": "center",
                        gap: "12px",
                        padding: "10px 0",
                        "border-bottom": "1px solid var(--border-secondary)",
                      }}
                    >
                      <label
                        class="toggle-switch"
                        style={{ "flex-shrink": "0" }}
                      >
                        <input
                          type="checkbox"
                          checked={ruleOverrides()[rule.id] ?? rule.enabled}
                          onChange={() => toggleRule(rule.id)}
                        />
                        <span class="toggle-slider" />
                      </label>
                      <div style={{ flex: "1", "min-width": "0" }}>
                        <div style={{ display: "flex", "align-items": "center", gap: "8px" }}>
                          <span style={{ "font-size": "13px", "font-weight": "600", color: ruleOverrides()[rule.id] ? "var(--text-primary)" : "var(--text-muted)" }}>
                            {rule.name}
                          </span>
                          <span
                            style={{
                              "font-size": "10px",
                              "font-weight": "700",
                              padding: "1px 6px",
                              "border-radius": "4px",
                              "text-transform": "uppercase",
                              background: severityColor(rule.severity),
                              color: "#fff",
                            }}
                          >
                            {rule.severity}
                          </span>
                        </div>
                        <div style={{ "font-size": "11px", color: "var(--text-muted)", "margin-top": "2px" }}>
                          {rule.description}
                        </div>
                      </div>
                    </div>
                  )}
                </For>
              </div>
            </div>
          )}
        </For>
      </div>

      {/* Pricing Reference */}
      <div class="card" style={{ "margin-top": "16px" }}>
        <div style={{ display: "flex", "align-items": "center", "justify-content": "space-between" }}>
          <div class="card-title" style={{ "margin-bottom": "0" }}>GCP Pricing Reference</div>
          <button class="btn btn-sm" onClick={() => setShowPricing(!showPricing())}>
            {showPricing() ? "Hide" : "Show"}
          </button>
        </div>
        <Show when={showPricing()}>
          <div style={{ "margin-top": "12px" }}>
            <For each={groupedPricing()}>
              {([group, entries]) => (
                <div style={{ "margin-bottom": "12px" }}>
                  <div style={{ "font-size": "11px", "font-weight": "600", color: "var(--text-secondary)", "margin-bottom": "4px" }}>
                    {group}
                  </div>
                  <table class="resource-table" style={{ "font-size": "11px" }}>
                    <thead>
                      <tr>
                        <th>SKU</th>
                        <th>Unit</th>
                        <th style={{ "text-align": "right" }}>Price</th>
                      </tr>
                    </thead>
                    <tbody>
                      <For each={entries}>
                        {(entry) => (
                          <tr>
                            <td>{entry.sku}</td>
                            <td style={{ color: "var(--text-muted)" }}>{entry.unit}</td>
                            <td style={{ "text-align": "right", "font-weight": "600" }}>
                              ${entry.price_per_unit < 0.01 ? entry.price_per_unit.toFixed(8) : entry.price_per_unit.toFixed(2)}
                            </td>
                          </tr>
                        )}
                      </For>
                    </tbody>
                  </table>
                </div>
              )}
            </For>
          </div>
        </Show>
      </div>

      {/* App Info */}
      <div class="card" style={{ "margin-top": "16px" }}>
        <div class="card-title">About</div>
        <div style={{ "font-size": "12px", color: "var(--text-secondary)", display: "flex", "flex-direction": "column", gap: "4px" }}>
          <div>r3x-cloud v0.1.0</div>
          <div>Cloud resource explorer and waste detector</div>
          <div style={{ color: "var(--text-muted)" }}>Data stays local. Read-only cloud access.</div>
        </div>
      </div>
    </div>
  );
}

function formatType(t: string): string {
  return t.split("_").map((w) => w.charAt(0).toUpperCase() + w.slice(1)).join(" ");
}
