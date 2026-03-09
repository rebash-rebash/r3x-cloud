import { createSignal, For, Show } from "solid-js";
import {
  accounts,
  activeAccountId,
  addAccount,
  removeAccount,
  setActiveAccountId,
} from "../stores/cloud";
import { testConnection } from "../lib/ipc";

export default function AccountManager() {
  const [showForm, setShowForm] = createSignal(false);
  const [provider, setProvider] = createSignal<"gcp" | "aws" | "azure">("gcp");
  const [displayName, setDisplayName] = createSignal("");
  const [projectId, setProjectId] = createSignal("");
  const [testing, setTesting] = createSignal(false);
  const [testResult, setTestResult] = createSignal<string | null>(null);
  const [testError, setTestError] = createSignal<string | null>(null);
  const [saving, setSaving] = createSignal(false);

  const handleTest = async () => {
    setTesting(true);
    setTestResult(null);
    setTestError(null);
    try {
      const result = await testConnection(provider(), projectId() || null, {});
      setTestResult(result);
    } catch (e) {
      setTestError(String(e));
    } finally {
      setTesting(false);
    }
  };

  const handleAdd = async () => {
    if (!displayName()) return;
    setSaving(true);
    try {
      await addAccount(provider(), displayName(), projectId() || null, {});
      setShowForm(false);
      setDisplayName("");
      setProjectId("");
      setTestResult(null);
      setTestError(null);
    } catch {
      // error is set in store
    } finally {
      setSaving(false);
    }
  };

  const handleRemove = async (id: string) => {
    await removeAccount(id);
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
        <h2 style={{ "font-size": "18px" }}>Accounts</h2>
        <button class="btn btn-primary btn-sm" onClick={() => setShowForm(!showForm())}>
          {showForm() ? "Cancel" : "+ Add Account"}
        </button>
      </div>

      {/* Existing accounts */}
      <div class="account-list">
        <For each={accounts()}>
          {(account) => (
            <div
              class={`account-card ${activeAccountId() === account.id ? "active" : ""}`}
              onClick={() => setActiveAccountId(account.id)}
            >
              <span
                class={`provider-${account.provider}`}
                style={{ "font-weight": "700", "font-size": "12px" }}
              >
                {account.provider.toUpperCase()}
              </span>
              <div class="account-card-info">
                <div class="account-card-name">{account.display_name}</div>
                <div class="account-card-detail">
                  {account.project_id || account.id}
                </div>
              </div>
              <button
                class="btn btn-danger btn-sm"
                onClick={(e) => {
                  e.stopPropagation();
                  handleRemove(account.id);
                }}
              >
                Remove
              </button>
            </div>
          )}
        </For>

        <Show when={accounts().length === 0 && !showForm()}>
          <div class="empty-state">
            <div class="empty-state-title">No accounts configured</div>
            <p>Add a cloud account to start scanning resources.</p>
          </div>
        </Show>
      </div>

      {/* Add account form */}
      <Show when={showForm()}>
        <div class="card">
          <div class="card-title">Add Cloud Account</div>
          <div class="account-form">
            <div class="form-group">
              <label>Provider</label>
              <select
                value={provider()}
                onChange={(e) =>
                  setProvider(e.currentTarget.value as "gcp" | "aws" | "azure")
                }
              >
                <option value="gcp">Google Cloud (GCP)</option>
                <option value="aws" disabled>
                  AWS (coming soon)
                </option>
                <option value="azure" disabled>
                  Azure (coming soon)
                </option>
              </select>
            </div>

            <div class="form-group">
              <label>Display Name</label>
              <input
                type="text"
                placeholder="My GCP Project"
                value={displayName()}
                onInput={(e) => setDisplayName(e.currentTarget.value)}
              />
            </div>

            <div class="form-group">
              <label>Project ID</label>
              <input
                type="text"
                placeholder="my-gcp-project-id"
                value={projectId()}
                onInput={(e) => setProjectId(e.currentTarget.value)}
              />
            </div>

            <div style={{ display: "flex", gap: "8px" }}>
              <button
                class="btn"
                onClick={handleTest}
                disabled={testing() || !projectId()}
              >
                {testing() ? "Testing..." : "Test Connection"}
              </button>
              <button
                class="btn btn-primary"
                onClick={handleAdd}
                disabled={saving() || !displayName() || !projectId()}
              >
                {saving() ? "Adding..." : "Add Account"}
              </button>
            </div>

            <Show when={testResult()}>
              <div
                style={{
                  color: "var(--color-success)",
                  "font-size": "12px",
                }}
              >
                {testResult()}
              </div>
            </Show>
            <Show when={testError()}>
              <div
                style={{
                  color: "var(--color-danger)",
                  "font-size": "12px",
                }}
              >
                {testError()}
              </div>
            </Show>
          </div>
        </div>
      </Show>
    </div>
  );
}
