import { createSignal } from "solid-js";
import type { CloudAccount } from "../lib/types";
import * as ipc from "../lib/ipc";

const [accounts, setAccounts] = createSignal<CloudAccount[]>([]);
const [activeAccountId, setActiveAccountId] = createSignal<string | null>(null);
const [loading, setLoading] = createSignal(false);
const [error, setError] = createSignal<string | null>(null);

export { accounts, activeAccountId, loading, error };

export function activeAccount() {
  const id = activeAccountId();
  return accounts().find((a) => a.id === id) || null;
}

export async function loadAccounts() {
  setLoading(true);
  setError(null);
  try {
    const result = await ipc.listAccounts();
    setAccounts(result);
    if (result.length > 0 && !activeAccountId()) {
      setActiveAccountId(result[0].id);
    }
  } catch (e) {
    setError(String(e));
  } finally {
    setLoading(false);
  }
}

export async function addAccount(
  provider: "gcp" | "aws" | "azure",
  displayName: string,
  projectId: string | null,
  config: Record<string, unknown>,
) {
  setError(null);
  try {
    const account = await ipc.addAccount(provider, displayName, projectId, config);
    setAccounts((prev) => [...prev, account]);
    setActiveAccountId(account.id);
    return account;
  } catch (e) {
    setError(String(e));
    throw e;
  }
}

export async function removeAccount(id: string) {
  setError(null);
  try {
    await ipc.removeAccount(id);
    setAccounts((prev) => prev.filter((a) => a.id !== id));
    if (activeAccountId() === id) {
      const remaining = accounts();
      setActiveAccountId(remaining.length > 0 ? remaining[0].id : null);
    }
  } catch (e) {
    setError(String(e));
  }
}

export { setActiveAccountId };
