import { createSignal, createMemo, For, onMount, onCleanup, Show } from "solid-js";
import type { ViewKind } from "../lib/types";
import { activeAccount } from "../stores/cloud";
import { startScan } from "../stores/scan";
import { runAnalysis } from "../stores/analysis";
import { toggleTheme } from "../stores/theme";

interface Command {
  id: string;
  label: string;
  shortcut?: string;
  action: () => void;
}

interface Props {
  open: boolean;
  onClose: () => void;
  onNavigate: (view: ViewKind) => void;
  onExport: () => void;
}

export default function CommandPalette(props: Props) {
  const [query, setQuery] = createSignal("");
  let inputRef: HTMLInputElement | undefined;

  const commands = (): Command[] => [
    { id: "dashboard", label: "Go to Dashboard", shortcut: "d", action: () => { props.onNavigate("dashboard"); props.onClose(); } },
    { id: "resources", label: "Go to Resources", shortcut: "r", action: () => { props.onNavigate("resources"); props.onClose(); } },
    { id: "findings", label: "Go to Findings", shortcut: "f", action: () => { props.onNavigate("recommendations"); props.onClose(); } },
    { id: "scan-view", label: "Go to Scan", action: () => { props.onNavigate("scan"); props.onClose(); } },
    { id: "accounts", label: "Go to Accounts", shortcut: "a", action: () => { props.onNavigate("accounts"); props.onClose(); } },
    { id: "history", label: "Go to History", shortcut: "h", action: () => { props.onNavigate("history"); props.onClose(); } },
    { id: "map", label: "Go to Infra Map", shortcut: "m", action: () => { props.onNavigate("map"); props.onClose(); } },
    { id: "diff", label: "Go to Scan Diff", action: () => { props.onNavigate("diff"); props.onClose(); } },
    { id: "settings", label: "Go to Settings", action: () => { props.onNavigate("settings"); props.onClose(); } },
    { id: "scan", label: "Run Scan", shortcut: "s", action: () => { const acc = activeAccount(); if (acc) startScan(acc.id); props.onClose(); } },
    { id: "analyze", label: "Run Analysis", action: () => { const acc = activeAccount(); if (acc) runAnalysis(acc.id); props.onClose(); } },
    { id: "theme", label: "Toggle Theme", shortcut: "t", action: () => { toggleTheme(); props.onClose(); } },
    { id: "export", label: "Export Data (CSV/JSON)", action: () => { props.onExport(); props.onClose(); } },
  ];

  const filtered = createMemo(() => {
    const q = query().toLowerCase();
    if (!q) return commands();
    return commands().filter((c) => c.label.toLowerCase().includes(q));
  });

  const [selectedIdx, setSelectedIdx] = createSignal(0);

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIdx((i) => Math.min(i + 1, filtered().length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIdx((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      const cmd = filtered()[selectedIdx()];
      if (cmd) cmd.action();
    } else if (e.key === "Escape") {
      props.onClose();
    }
  };

  onMount(() => {
    if (props.open) {
      setTimeout(() => inputRef?.focus(), 0);
    }
  });

  // Reset when opened
  const focusInput = () => {
    setQuery("");
    setSelectedIdx(0);
    setTimeout(() => inputRef?.focus(), 0);
  };

  // Watch for open changes
  createMemo(() => {
    if (props.open) focusInput();
  });

  return (
    <Show when={props.open}>
      <div class="palette-overlay" onClick={props.onClose} />
      <div class="palette" onKeyDown={handleKeyDown}>
        <input
          ref={inputRef}
          class="palette-input"
          type="text"
          placeholder="Type a command..."
          value={query()}
          onInput={(e) => { setQuery(e.currentTarget.value); setSelectedIdx(0); }}
        />
        <div class="palette-list">
          <For each={filtered()}>
            {(cmd, idx) => (
              <div
                class={`palette-item ${selectedIdx() === idx() ? "selected" : ""}`}
                onClick={() => cmd.action()}
                onMouseEnter={() => setSelectedIdx(idx())}
              >
                <span>{cmd.label}</span>
                <Show when={cmd.shortcut}>
                  <kbd>{cmd.shortcut}</kbd>
                </Show>
              </div>
            )}
          </For>
          <Show when={filtered().length === 0}>
            <div class="palette-empty">No matching commands</div>
          </Show>
        </div>
      </div>
    </Show>
  );
}
