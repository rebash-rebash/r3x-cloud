import { createSignal, onMount, onCleanup, Show } from "solid-js";
import "./styles/global.css";
import type { ViewKind, ResourceType } from "./lib/types";
import { loadAccounts, activeAccount } from "./stores/cloud";
import { setupScanListeners, startScan } from "./stores/scan";
import { toggleTheme } from "./stores/theme";
import Sidebar from "./components/Sidebar";
import Header from "./components/Header";
import KeyboardBar from "./components/KeyboardBar";
import Dashboard from "./components/Dashboard";
import ResourceExplorer from "./components/ResourceExplorer";
import ScanPanel from "./components/ScanPanel";
import AccountManager from "./components/AccountManager";
import RecommendationsPanel from "./components/RecommendationsPanel";
import CommandPalette from "./components/CommandPalette";
import ExportDialog from "./components/ExportDialog";
import ScanHistory from "./components/ScanHistory";
import InfraMap from "./components/InfraMap";
import SettingsPanel from "./components/SettingsPanel";
import ScanDiff from "./components/ScanDiff";

function App() {
  const [activeView, setActiveView] = createSignal<ViewKind>("dashboard");
  const [paletteOpen, setPaletteOpen] = createSignal(false);
  const [exportOpen, setExportOpen] = createSignal(false);
  const [typeFilter, setTypeFilter] = createSignal<ResourceType | null>(null);

  onMount(() => {
    loadAccounts();
    setupScanListeners();
  });

  const navigateToResources = (type?: ResourceType) => {
    setTypeFilter(type ?? null);
    setActiveView("resources");
  };

  // Global keyboard shortcuts
  const handleKeyDown = (e: KeyboardEvent) => {
    // Close palette/export on Escape
    if (e.key === "Escape") {
      if (paletteOpen()) { setPaletteOpen(false); return; }
      if (exportOpen()) { setExportOpen(false); return; }
    }

    // Don't handle shortcuts when in input fields (except Escape)
    const tag = (e.target as HTMLElement).tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") {
      if (e.key === "Escape") {
        (e.target as HTMLElement).blur();
      }
      return;
    }

    // Don't handle shortcuts when palette is open
    if (paletteOpen()) return;

    switch (e.key) {
      case ":":
        e.preventDefault();
        setPaletteOpen(true);
        break;
      case "/":
        e.preventDefault();
        setActiveView("resources");
        setTimeout(() => document.getElementById("search-input")?.focus(), 50);
        break;
      case "d":
        setActiveView("dashboard");
        break;
      case "s":
        {
          const acc = activeAccount();
          if (acc) startScan(acc.id);
        }
        break;
      case "t":
        toggleTheme();
        break;
      case "r":
        navigateToResources();
        break;
      case "f":
        setActiveView("recommendations");
        break;
      case "a":
        setActiveView("accounts");
        break;
      case "h":
        setActiveView("history");
        break;
      case "m":
        setActiveView("map");
        break;
      case "e":
        setExportOpen(true);
        break;
      case "Escape":
        break;
    }
  };

  onMount(() => {
    document.addEventListener("keydown", handleKeyDown);
  });

  onCleanup(() => {
    document.removeEventListener("keydown", handleKeyDown);
  });

  return (
    <>
      <Header />
      <div class="app-layout">
        <Sidebar
          activeView={activeView()}
          onNavigate={setActiveView}
          onFilterResources={navigateToResources}
          activeTypeFilter={typeFilter()}
        />
        <main class="main-content">
          <Show when={activeView() === "dashboard"}>
            <Dashboard />
          </Show>
          <Show when={activeView() === "resources"}>
            <ResourceExplorer
              typeFilter={typeFilter()}
              onClearFilter={() => setTypeFilter(null)}
            />
          </Show>
          <Show when={activeView() === "scan"}>
            <ScanPanel />
          </Show>
          <Show when={activeView() === "accounts"}>
            <AccountManager />
          </Show>
          <Show when={activeView() === "settings"}>
            <SettingsPanel />
          </Show>
          <Show when={activeView() === "diff"}>
            <ScanDiff />
          </Show>
          <Show when={activeView() === "recommendations"}>
            <RecommendationsPanel />
          </Show>
          <Show when={activeView() === "history"}>
            <ScanHistory />
          </Show>
          <Show when={activeView() === "map"}>
            <InfraMap />
          </Show>
        </main>
      </div>
      <KeyboardBar />
      <CommandPalette
        open={paletteOpen()}
        onClose={() => setPaletteOpen(false)}
        onNavigate={setActiveView}
        onExport={() => setExportOpen(true)}
      />
      <ExportDialog
        open={exportOpen()}
        onClose={() => setExportOpen(false)}
      />
    </>
  );
}

export default App;
