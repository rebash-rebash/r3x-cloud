import { Show, For, createSignal, onMount } from "solid-js";
import { activeAccount, accounts, addAccount, setActiveAccountId } from "../stores/cloud";
import { scanning, startScan } from "../stores/scan";
import { toggleTheme, theme } from "../stores/theme";
import type { GcpProject } from "../lib/types";
import * as ipc from "../lib/ipc";

export default function Header() {
  const account = activeAccount;
  const [projects, setProjects] = createSignal<GcpProject[]>([]);
  const [loadingProjects, setLoadingProjects] = createSignal(false);

  onMount(async () => {
    setLoadingProjects(true);
    try {
      const result = await ipc.listGcpProjects();
      setProjects(result);
    } catch {
      // gcloud not available or no projects
    } finally {
      setLoadingProjects(false);
    }
  });

  const handleProjectChange = async (projectId: string) => {
    if (!projectId) return;
    const existing = accounts().find((a) => a.project_id === projectId);
    if (existing) {
      setActiveAccountId(existing.id);
      return;
    }
    const project = projects().find((p) => p.project_id === projectId);
    const displayName = project?.name || projectId;
    try {
      await addAccount("gcp", displayName, projectId, {});
    } catch {
      // user will see error in accounts page
    }
  };

  const handleScan = () => {
    const acc = account();
    if (acc) startScan(acc.id);
  };

  const providerLabel = () => {
    const acc = account();
    if (!acc) return null;
    return acc.provider.toUpperCase();
  };

  return (
    <div class="header-wrapper">
      {/* Row 1: Brand ribbon */}
      <div class="header-ribbon">
        <div style={{ display: "flex", "align-items": "center", gap: "10px" }}>
          <div class="header-brand">r3x-cloud</div>
          <span style={{ "font-size": "10px", color: "var(--text-muted)", "letter-spacing": "0.5px", opacity: "0.7" }}>
            Read-only &middot; Your data stays local
          </span>
        </div>
        <div class="header-ribbon-right">
          <Show when={account()}>
            <span class={`header-provider provider-${account()!.provider}`}>
              {providerLabel()}
            </span>
            <span class="header-account-name">{account()!.display_name}</span>
          </Show>
          <Show when={!account()}>
            <span class="header-account-name" style={{ color: "var(--text-muted)" }}>No account selected</span>
          </Show>
          <button class="btn btn-sm" onClick={toggleTheme}>
            {theme() === "dark" ? "Light" : "Dark"}
          </button>
        </div>
      </div>

      {/* Row 2: Toolbar */}
      <div class="header-toolbar">
        <div class="header-toolbar-right">
          <Show when={projects().length > 0}>
            <select
              class="project-select"
              value={account()?.project_id || ""}
              onChange={(e) => handleProjectChange(e.currentTarget.value)}
            >
              <option value="" disabled>
                {loadingProjects() ? "Loading..." : "Select project"}
              </option>
              <For each={projects()}>
                {(project) => (
                  <option value={project.project_id}>
                    {project.name} ({project.project_id})
                  </option>
                )}
              </For>
            </select>
          </Show>

          <button
            class="btn btn-primary btn-sm"
            onClick={handleScan}
            disabled={scanning() || !account()}
          >
            {scanning() ? "Scanning..." : "Scan"}
          </button>
        </div>
      </div>
    </div>
  );
}
