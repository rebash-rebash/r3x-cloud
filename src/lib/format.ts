export function formatCurrency(amount: number | null | undefined): string {
  if (amount == null) return "-";
  return `$${amount.toFixed(2)}`;
}

export function formatDate(date: string | null | undefined): string {
  if (!date) return "-";
  try {
    return new Date(date).toLocaleDateString("en-US", {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  } catch {
    return date;
  }
}

export function formatResourceType(type: string): string {
  return type
    .split("_")
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ");
}

export function formatStatus(status: string): string {
  return status.charAt(0).toUpperCase() + status.slice(1).toLowerCase();
}

export function statusColor(status: string): string {
  const s = status.toUpperCase();
  if (s === "RUNNING" || s === "ACTIVE") return "var(--color-success)";
  if (s === "TERMINATED" || s === "STOPPED" || s === "DEALLOCATED")
    return "var(--color-danger)";
  if (s === "STAGING" || s === "PROVISIONING" || s === "PENDING")
    return "var(--color-warning)";
  return "var(--color-muted)";
}
