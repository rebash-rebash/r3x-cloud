import { For, Show } from "solid-js";
import { scanning, scanProgress, scanError, lastScanResult, startScan } from "../stores/scan";
import { activeAccount } from "../stores/cloud";

export default function ScanPanel() {
  const handleScan = () => {
    const acc = activeAccount();
    if (acc) startScan(acc.id);
  };

  return (
    <div>
      <div
        style={{
          display: "flex",
          "align-items": "center",
          gap: "12px",
          "margin-bottom": "16px",
        }}
      >
        <h2 style={{ "font-size": "18px" }}>Scan</h2>
        <button
          class="btn btn-primary"
          onClick={handleScan}
          disabled={scanning() || !activeAccount()}
        >
          {scanning() ? "Scanning..." : "Start Scan"}
        </button>
      </div>

      <Show when={!activeAccount()}>
        <div class="empty-state">
          <div class="empty-state-title">No account selected</div>
          <p>Add and select a cloud account first.</p>
        </div>
      </Show>

      <Show when={scanError()}>
        <div
          class="card"
          style={{
            "border-color": "var(--color-danger)",
            "margin-bottom": "16px",
          }}
        >
          <div class="card-title" style={{ color: "var(--color-danger)" }}>
            Scan Error
          </div>
          <p style={{ "font-size": "12px" }}>{scanError()}</p>
        </div>
      </Show>

      <Show when={scanProgress().length > 0}>
        <div class="card" style={{ "margin-bottom": "16px" }}>
          <div class="card-title">Scan Progress</div>
          <For each={scanProgress()}>
            {(progress) => (
              <div class="scan-item">
                <div class="scan-item-type">{progress.resource_type}</div>
                <div class="scan-item-bar">
                  <div class="progress-bar">
                    <div
                      class="progress-bar-fill"
                      style={{
                        width:
                          progress.status === "completed" ? "100%" : "50%",
                        background:
                          progress.status === "failed"
                            ? "var(--color-danger)"
                            : "var(--color-accent)",
                      }}
                    />
                  </div>
                </div>
                <div class="scan-item-count">
                  {progress.found} found
                </div>
              </div>
            )}
          </For>
        </div>
      </Show>

      <Show when={lastScanResult()}>
        <div class="card">
          <div class="card-title">Last Scan Result</div>
          <p style={{ "font-size": "13px" }}>
            Found{" "}
            <strong>{lastScanResult()!.total_resources}</strong> resources
          </p>
          <p
            style={{ "font-size": "11px", color: "var(--text-muted)" }}
          >
            Scan ID: {lastScanResult()!.scan_id}
          </p>
        </div>
      </Show>
    </div>
  );
}
