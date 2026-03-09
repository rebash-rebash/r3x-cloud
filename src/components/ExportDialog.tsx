import { Show, createSignal } from "solid-js";
import { resources } from "../stores/scan";
import { analysis } from "../stores/analysis";
import { activeAccount } from "../stores/cloud";
import * as ipc from "../lib/ipc";

interface Props {
  open: boolean;
  onClose: () => void;
}

export default function ExportDialog(props: Props) {
  const [exporting, setExporting] = createSignal(false);
  const [result, setResult] = createSignal<string | null>(null);
  const [error, setError] = createSignal<string | null>(null);
  const [format, setFormat] = createSignal<"csv" | "json">("csv");
  const [exportType, setExportType] = createSignal<"resources" | "findings" | "all">("resources");

  const handleExport = async () => {
    const account = activeAccount();
    if (!account) return;

    setExporting(true);
    setResult(null);
    setError(null);

    try {
      const path = await ipc.exportToFile(account.id, format(), exportType());
      setResult(path);
      setTimeout(() => setResult(null), 5000);
    } catch (e) {
      setError(String(e));
    } finally {
      setExporting(false);
    }
  };

  return (
    <Show when={props.open}>
      <div class="palette-overlay" onClick={props.onClose} />
      <div class="export-dialog">
        <h3 style={{ "font-size": "16px", "margin-bottom": "16px" }}>Export Data</h3>

        <div class="form-group" style={{ "margin-bottom": "12px" }}>
          <label style={{ "font-size": "12px", color: "var(--text-muted)", "margin-bottom": "4px", display: "block" }}>Format</label>
          <div style={{ display: "flex", gap: "8px" }}>
            <button
              class={`btn ${format() === "csv" ? "btn-primary" : ""}`}
              onClick={() => setFormat("csv")}
            >
              CSV
            </button>
            <button
              class={`btn ${format() === "json" ? "btn-primary" : ""}`}
              onClick={() => setFormat("json")}
            >
              JSON
            </button>
          </div>
        </div>

        <div class="form-group" style={{ "margin-bottom": "16px" }}>
          <label style={{ "font-size": "12px", color: "var(--text-muted)", "margin-bottom": "4px", display: "block" }}>Content</label>
          <div style={{ display: "flex", gap: "8px" }}>
            <button
              class={`btn ${exportType() === "resources" ? "btn-primary" : ""}`}
              onClick={() => setExportType("resources")}
            >
              Resources
            </button>
            <button
              class={`btn ${exportType() === "findings" ? "btn-primary" : ""}`}
              onClick={() => setExportType("findings")}
            >
              Findings
            </button>
            <button
              class={`btn ${exportType() === "all" ? "btn-primary" : ""}`}
              onClick={() => setExportType("all")}
            >
              All
            </button>
          </div>
        </div>

        <div style={{ "font-size": "12px", color: "var(--text-muted)", "margin-bottom": "16px" }}>
          <Show when={exportType() === "resources"}>
            Export {resources().length} resources as {format().toUpperCase()} to ~/Downloads.
          </Show>
          <Show when={exportType() === "findings"}>
            Export {analysis()?.total_findings ?? 0} findings as {format().toUpperCase()} to ~/Downloads.
          </Show>
          <Show when={exportType() === "all"}>
            Export {resources().length} resources and {analysis()?.total_findings ?? 0} findings as {format().toUpperCase()} to ~/Downloads.
          </Show>
        </div>

        <Show when={result()}>
          <div style={{ "font-size": "11px", color: "var(--color-success)", "margin-bottom": "12px", "word-break": "break-all" }}>
            Saved to {result()}
          </div>
        </Show>

        <Show when={error()}>
          <div style={{ "font-size": "11px", color: "var(--color-danger)", "margin-bottom": "12px" }}>
            {error()}
          </div>
        </Show>

        <div style={{ display: "flex", gap: "8px", "justify-content": "flex-end" }}>
          <button class="btn" onClick={props.onClose}>Cancel</button>
          <button
            class="btn btn-primary"
            onClick={handleExport}
            disabled={resources().length === 0 || exporting() || !activeAccount()}
          >
            {exporting() ? "Exporting..." : result() ? "Saved!" : "Export"}
          </button>
        </div>
      </div>
    </Show>
  );
}
