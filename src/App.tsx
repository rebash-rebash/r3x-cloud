import { createSignal, onMount, onCleanup, Show } from "solid-js";
import "./styles/global.css";
import type { ViewKind } from "./lib/types";
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

function App() {
  const [activeView, setActiveView] = createSignal<ViewKind>("dashboard");
  const [searchQuery, setSearchQuery] = createSignal("");

  onMount(() => {
    loadAccounts();
    setupScanListeners();
  });

  // Global keyboard shortcuts
  const handleKeyDown = (e: KeyboardEvent) => {
    const tag = (e.target as HTMLElement).tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") {
      if (e.key === "Escape") {
        (e.target as HTMLElement).blur();
      }
      return;
    }

    switch (e.key) {
      case "/":
        e.preventDefault();
        document.getElementById("search-input")?.focus();
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
        setActiveView("resources");
        break;
      case "a":
        setActiveView("accounts");
        break;
      case "Escape":
        setSearchQuery("");
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
      <Header searchQuery={searchQuery()} onSearch={setSearchQuery} />
      <div class="app-layout">
        <Sidebar activeView={activeView()} onNavigate={setActiveView} />
        <main class="main-content">
          <Show when={activeView() === "dashboard"}>
            <Dashboard />
          </Show>
          <Show when={activeView() === "resources"}>
            <ResourceExplorer searchQuery={searchQuery()} />
          </Show>
          <Show when={activeView() === "scan"}>
            <ScanPanel />
          </Show>
          <Show when={activeView() === "accounts"}>
            <AccountManager />
          </Show>
          <Show when={activeView() === "settings"}>
            <div>
              <h2 style={{ "font-size": "18px", "margin-bottom": "16px" }}>
                Settings
              </h2>
              <p style={{ color: "var(--text-muted)", "font-size": "13px" }}>
                Rule configuration and preferences coming in Phase 2.
              </p>
            </div>
          </Show>
          <Show when={activeView() === "recommendations"}>
            <div>
              <h2 style={{ "font-size": "18px", "margin-bottom": "16px" }}>
                Recommendations
              </h2>
              <p style={{ color: "var(--text-muted)", "font-size": "13px" }}>
                Cleanup recommendations coming in Phase 2.
              </p>
            </div>
          </Show>
        </main>
      </div>
      <KeyboardBar />
    </>
  );
}

export default App;
