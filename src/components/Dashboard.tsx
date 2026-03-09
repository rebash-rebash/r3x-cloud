import { Show, createMemo } from "solid-js";
import { resources } from "../stores/scan";
import { activeAccount } from "../stores/cloud";
import { formatCurrency } from "../lib/format";

export default function Dashboard() {
  const totalResources = () => resources().length;

  const totalMonthlyCost = createMemo(() =>
    resources().reduce((sum, r) => sum + (r.monthly_cost || 0), 0),
  );

  const stoppedVMs = createMemo(
    () =>
      resources().filter(
        (r) =>
          r.resource_type === "virtual_machine" &&
          r.status.toUpperCase() === "TERMINATED",
      ).length,
  );

  const runningVMs = createMemo(
    () =>
      resources().filter(
        (r) =>
          r.resource_type === "virtual_machine" &&
          r.status.toUpperCase() === "RUNNING",
      ).length,
  );

  const regionCount = createMemo(
    () => new Set(resources().map((r) => r.region)).size,
  );

  return (
    <div>
      <h2 style={{ "margin-bottom": "16px", "font-size": "18px" }}>
        Dashboard
      </h2>

      <Show
        when={activeAccount()}
        fallback={
          <div class="empty-state">
            <div class="empty-state-title">No account configured</div>
            <p>Add a cloud account to get started.</p>
          </div>
        }
      >
        <div class="dashboard-grid">
          <div class="card">
            <div class="card-title">Total Resources</div>
            <div class="card-value">{totalResources()}</div>
          </div>

          <div class="card">
            <div class="card-title">Est. Monthly Cost</div>
            <div class="card-value">{formatCurrency(totalMonthlyCost())}</div>
          </div>

          <div class="card">
            <div class="card-title">Running VMs</div>
            <div class="card-value" style={{ color: "var(--color-success)" }}>
              {runningVMs()}
            </div>
          </div>

          <div class="card">
            <div class="card-title">Stopped VMs</div>
            <div class="card-value" style={{ color: "var(--color-danger)" }}>
              {stoppedVMs()}
            </div>
          </div>

          <div class="card">
            <div class="card-title">Regions</div>
            <div class="card-value">{regionCount()}</div>
          </div>
        </div>

        <Show when={totalResources() === 0}>
          <div class="empty-state">
            <div class="empty-state-title">No scan data</div>
            <p>
              Click <kbd>Scan</kbd> or press <kbd>s</kbd> to scan your cloud
              resources.
            </p>
          </div>
        </Show>
      </Show>
    </div>
  );
}
