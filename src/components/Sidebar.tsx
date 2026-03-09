import { For, Show } from "solid-js";
import type { ViewKind } from "../lib/types";
import { accounts, activeAccountId, setActiveAccountId } from "../stores/cloud";
import { resources } from "../stores/scan";

interface Props {
  activeView: ViewKind;
  onNavigate: (view: ViewKind) => void;
}

export default function Sidebar(props: Props) {
  const resourceCount = (type: string) =>
    resources().filter((r) => r.resource_type === type).length;

  const totalResources = () => resources().length;

  return (
    <nav class="sidebar">
      <div class="sidebar-header">r3x-cloud</div>

      {/* Account selector */}
      <Show when={accounts().length > 0}>
        <div class="sidebar-section">
          <div class="sidebar-section-title">Account</div>
          <For each={accounts()}>
            {(account) => (
              <div
                class={`sidebar-item ${activeAccountId() === account.id ? "active" : ""}`}
                onClick={() => setActiveAccountId(account.id)}
              >
                <span class={`provider-${account.provider}`}>
                  {account.provider.toUpperCase()}
                </span>
                <span>{account.display_name}</span>
              </div>
            )}
          </For>
        </div>
      </Show>

      {/* Navigation */}
      <div class="sidebar-section">
        <div class="sidebar-section-title">Navigation</div>
        <div
          class={`sidebar-item ${props.activeView === "dashboard" ? "active" : ""}`}
          onClick={() => props.onNavigate("dashboard")}
        >
          Dashboard
        </div>
        <div
          class={`sidebar-item ${props.activeView === "resources" ? "active" : ""}`}
          onClick={() => props.onNavigate("resources")}
        >
          Resources
          <Show when={totalResources() > 0}>
            <span class="badge">{totalResources()}</span>
          </Show>
        </div>
        <div
          class={`sidebar-item ${props.activeView === "scan" ? "active" : ""}`}
          onClick={() => props.onNavigate("scan")}
        >
          Scan
        </div>
      </div>

      {/* Resource types */}
      <Show when={totalResources() > 0}>
        <div class="sidebar-section">
          <div class="sidebar-section-title">Compute</div>
          <div
            class="sidebar-item"
            onClick={() => props.onNavigate("resources")}
          >
            Virtual Machines
            <Show when={resourceCount("virtual_machine") > 0}>
              <span class="badge">{resourceCount("virtual_machine")}</span>
            </Show>
          </div>
        </div>
      </Show>

      {/* Settings */}
      <div class="sidebar-section" style={{ "margin-top": "auto" }}>
        <div
          class={`sidebar-item ${props.activeView === "accounts" ? "active" : ""}`}
          onClick={() => props.onNavigate("accounts")}
        >
          Accounts
        </div>
        <div
          class={`sidebar-item ${props.activeView === "settings" ? "active" : ""}`}
          onClick={() => props.onNavigate("settings")}
        >
          Settings
        </div>
      </div>
    </nav>
  );
}
