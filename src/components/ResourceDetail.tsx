import { createSignal, Show, For } from "solid-js";
import type { CloudResource } from "../lib/types";
import { formatCurrency, formatDate, formatResourceType, statusColor } from "../lib/format";
import { analysis } from "../stores/analysis";

interface Props {
  resource: CloudResource | null;
  onClose: () => void;
}

function getActions(resource: CloudResource): { label: string; description: string; danger: boolean }[] {
  const type = resource.resource_type;
  const status = resource.status.toUpperCase();
  const actions: { label: string; description: string; danger: boolean }[] = [];

  if (type === "virtual_machine") {
    if (status === "TERMINATED" || status === "STOPPED") {
      actions.push({ label: "Delete VM", description: `gcloud compute instances delete ${resource.name} --zone=${resource.region} --quiet`, danger: true });
      actions.push({ label: "Start VM", description: `gcloud compute instances start ${resource.name} --zone=${resource.region}`, danger: false });
    }
    if (status === "RUNNING") {
      actions.push({ label: "Stop VM", description: `gcloud compute instances stop ${resource.name} --zone=${resource.region}`, danger: false });
    }
  }

  if (type === "disk" && status === "UNATTACHED") {
    actions.push({ label: "Snapshot & Delete", description: `gcloud compute disks snapshot ${resource.name} --zone=${resource.region} && gcloud compute disks delete ${resource.name} --zone=${resource.region} --quiet`, danger: true });
    actions.push({ label: "Delete Disk", description: `gcloud compute disks delete ${resource.name} --zone=${resource.region} --quiet`, danger: true });
  }

  if (type === "elastic_ip" && status === "RESERVED") {
    actions.push({ label: "Release IP", description: `gcloud compute addresses delete ${resource.name} --region=${resource.region} --quiet`, danger: true });
  }

  if (type === "snapshot") {
    actions.push({ label: "Delete Snapshot", description: `gcloud compute snapshots delete ${resource.name} --quiet`, danger: true });
  }

  if (type === "load_balancer" && status === "NO_TARGET") {
    actions.push({ label: "Delete Forwarding Rule", description: `gcloud compute forwarding-rules delete ${resource.name} --region=${resource.region} --quiet`, danger: true });
  }

  if (type === "security_group" && status === "DISABLED") {
    actions.push({ label: "Delete Firewall Rule", description: `gcloud compute firewall-rules delete ${resource.name} --quiet`, danger: true });
  }

  if (type === "machine_image") {
    actions.push({ label: "Delete Image", description: `gcloud compute images delete ${resource.name} --quiet`, danger: true });
  }

  if (type === "storage_bucket") {
    actions.push({ label: "Delete Bucket", description: `gcloud storage rm -r gs://${resource.name}`, danger: true });
  }

  if (type === "cloud_sql_instance") {
    if (status === "STOPPED" || status === "SUSPENDED") {
      actions.push({ label: "Delete Instance", description: `gcloud sql instances delete ${resource.name} --quiet`, danger: true });
    }
  }

  if (type === "serverless_function") {
    actions.push({ label: "Delete Function", description: `gcloud functions delete ${resource.name} --region=${resource.region} --quiet`, danger: true });
  }

  if (type === "cloud_run_service") {
    actions.push({ label: "Delete Service", description: `gcloud run services delete ${resource.name} --region=${resource.region} --quiet`, danger: true });
  }

  if (type === "gke_cluster") {
    const loc = resource.region;
    actions.push({ label: "Delete Cluster", description: `gcloud container clusters delete ${resource.name} --location=${loc} --quiet`, danger: true });
    if (status === "RUNNING") {
      actions.push({ label: "Resize to 0", description: `gcloud container clusters resize ${resource.name} --location=${loc} --num-nodes=0 --quiet`, danger: false });
    }
  }

  if (type === "big_query_dataset") {
    actions.push({ label: "Delete Dataset", description: `bq rm -r -f ${resource.account_id}:${resource.name}`, danger: true });
  }

  if (type === "pub_sub_topic") {
    actions.push({ label: "Delete Topic", description: `gcloud pubsub topics delete ${resource.name} --quiet`, danger: true });
  }

  if (type === "pub_sub_subscription") {
    actions.push({ label: "Delete Subscription", description: `gcloud pubsub subscriptions delete ${resource.name} --quiet`, danger: true });
  }

  if (type === "spanner_instance") {
    actions.push({ label: "Delete Instance", description: `gcloud spanner instances delete ${resource.name} --quiet`, danger: true });
  }

  if (type === "memorystore_instance") {
    actions.push({ label: "Delete Instance", description: `gcloud redis instances delete ${resource.name} --region=${resource.region} --quiet`, danger: true });
  }

  if (type === "app_engine_version") {
    const service = (resource.metadata as Record<string, unknown>).service as string || "default";
    actions.push({ label: "Delete Version", description: `gcloud app versions delete ${resource.name} --service=${service} --quiet`, danger: true });
  }

  if (type === "nat_gateway") {
    const router = (resource.metadata as Record<string, unknown>).router_name as string || "";
    actions.push({ label: "Delete NAT", description: `gcloud compute routers nats delete ${resource.name} --router=${router} --region=${resource.region} --quiet`, danger: true });
  }

  if (type === "vpn_tunnel") {
    actions.push({ label: "Delete Tunnel", description: `gcloud compute vpn-tunnels delete ${resource.name} --region=${resource.region} --quiet`, danger: true });
  }

  if (type === "artifact_registry_repo") {
    actions.push({ label: "Delete Repo", description: `gcloud artifacts repositories delete ${resource.name} --location=${resource.region} --quiet`, danger: true });
  }

  if (type === "dataproc_cluster") {
    actions.push({ label: "Delete Cluster", description: `gcloud dataproc clusters delete ${resource.name} --region=${resource.region} --quiet`, danger: true });
  }

  if (type === "secret_manager_secret") {
    actions.push({ label: "Delete Secret", description: `gcloud secrets delete ${resource.name} --quiet`, danger: true });
  }

  if (type === "log_sink") {
    actions.push({ label: "Delete Sink", description: `gcloud logging sinks delete ${resource.name} --quiet`, danger: true });
  }

  return actions;
}

export default function ResourceDetail(props: Props) {
  const [copiedIdx, setCopiedIdx] = createSignal<number | null>(null);

  const resourceFindings = () => {
    const data = analysis();
    const r = props.resource;
    if (!data || !r) return [];
    return data.findings.filter((f) => f.resource_id === r.id);
  };

  const metadataEntries = () => {
    const r = props.resource;
    if (!r) return [];
    return Object.entries(r.metadata as Record<string, unknown>).filter(
      ([, v]) => v != null && v !== "" && v !== "null",
    );
  };

  const tagEntries = () => {
    const r = props.resource;
    if (!r) return [];
    return Object.entries(r.tags);
  };

  const actions = () => {
    const r = props.resource;
    if (!r) return [];
    return getActions(r);
  };

  const copyCommand = (cmd: string, idx: number) => {
    navigator.clipboard.writeText(cmd);
    setCopiedIdx(idx);
    setTimeout(() => setCopiedIdx(null), 1500);
  };

  return (
    <Show when={props.resource}>
      <div class="detail-overlay" onClick={props.onClose} />
      <div class="detail-panel">
        <div class="detail-header">
          <div style={{ flex: "1" }}>
            <div class="detail-title">{props.resource!.name}</div>
            <div class="detail-subtitle">
              {formatResourceType(props.resource!.resource_type)} &middot; {props.resource!.region}
            </div>
          </div>
          <button class="btn btn-sm" onClick={props.onClose}>
            ESC
          </button>
        </div>

        {/* Status & Cost */}
        <div class="detail-section">
          <div style={{ display: "flex", gap: "16px" }}>
            <div class="detail-stat">
              <div class="detail-stat-label">Status</div>
              <span
                class="status-badge"
                style={{
                  color: statusColor(props.resource!.status),
                  background: `${statusColor(props.resource!.status)}20`,
                }}
              >
                {props.resource!.status}
              </span>
            </div>
            <div class="detail-stat">
              <div class="detail-stat-label">Monthly Cost</div>
              <div style={{ "font-size": "18px", "font-weight": "700" }}>
                {formatCurrency(props.resource!.monthly_cost)}
              </div>
            </div>
            <div class="detail-stat">
              <div class="detail-stat-label">Provider</div>
              <div>{props.resource!.provider.toUpperCase()}</div>
            </div>
          </div>
        </div>

        {/* Actions */}
        <Show when={actions().length > 0}>
          <div class="detail-section">
            <div class="detail-section-title">Actions</div>
            <div style={{ display: "flex", "flex-direction": "column", gap: "6px" }}>
              <For each={actions()}>
                {(action, idx) => (
                  <div class="action-item">
                    <div style={{ display: "flex", "align-items": "center", gap: "8px", "margin-bottom": "4px" }}>
                      <span style={{ "font-size": "12px", "font-weight": "600", color: action.danger ? "var(--color-danger)" : "var(--text-primary)" }}>
                        {action.label}
                      </span>
                      <button
                        class="btn btn-sm"
                        style={{ "margin-left": "auto", "font-size": "10px" }}
                        onClick={() => copyCommand(action.description, idx())}
                      >
                        {copiedIdx() === idx() ? "Copied!" : "Copy"}
                      </button>
                    </div>
                    <code class="action-command">{action.description}</code>
                  </div>
                )}
              </For>
              <div style={{ "font-size": "10px", color: "var(--text-muted)", "margin-top": "4px", "font-style": "italic" }}>
                Copy and run in terminal. r3x-cloud is read-only.
              </div>
            </div>
          </div>
        </Show>

        {/* Dates */}
        <div class="detail-section">
          <div class="detail-section-title">Timeline</div>
          <div class="detail-row">
            <span class="detail-row-label">Created</span>
            <span>{formatDate(props.resource!.created_at)}</span>
          </div>
          <div class="detail-row">
            <span class="detail-row-label">Last Used</span>
            <span>{formatDate(props.resource!.last_used)}</span>
          </div>
        </div>

        {/* Findings */}
        <Show when={resourceFindings().length > 0}>
          <div class="detail-section">
            <div class="detail-section-title">
              Findings ({resourceFindings().length})
            </div>
            <For each={resourceFindings()}>
              {(finding) => (
                <div class="detail-finding">
                  <div style={{ display: "flex", "align-items": "center", gap: "8px" }}>
                    <span
                      class="status-badge"
                      style={{
                        color: finding.severity === "critical" || finding.severity === "high"
                          ? "var(--color-danger)" : "var(--color-warning)",
                        background: finding.severity === "critical" || finding.severity === "high"
                          ? "var(--color-danger)20" : "var(--color-warning)20",
                        "font-size": "10px",
                        "text-transform": "uppercase",
                      }}
                    >
                      {finding.severity}
                    </span>
                    <span style={{ "font-weight": "600", "font-size": "12px" }}>
                      {finding.rule_name}
                    </span>
                    <Show when={finding.estimated_monthly_savings > 0}>
                      <span style={{ "margin-left": "auto", color: "var(--color-success)", "font-weight": "600", "font-size": "12px" }}>
                        {formatCurrency(finding.estimated_monthly_savings)}/mo
                      </span>
                    </Show>
                  </div>
                  <div style={{ "font-size": "11px", color: "var(--text-muted)", "margin-top": "4px" }}>
                    {finding.recommendation}
                  </div>
                </div>
              )}
            </For>
          </div>
        </Show>

        {/* Metadata */}
        <Show when={metadataEntries().length > 0}>
          <div class="detail-section">
            <div class="detail-section-title">Metadata</div>
            <For each={metadataEntries()}>
              {([key, value]) => (
                <div class="detail-row">
                  <span class="detail-row-label">{key}</span>
                  <span class="detail-row-value">
                    {typeof value === "object" ? JSON.stringify(value) : String(value)}
                  </span>
                </div>
              )}
            </For>
          </div>
        </Show>

        {/* Tags */}
        <Show when={tagEntries().length > 0}>
          <div class="detail-section">
            <div class="detail-section-title">Tags</div>
            <div style={{ display: "flex", "flex-wrap": "wrap", gap: "4px" }}>
              <For each={tagEntries()}>
                {([key, value]) => (
                  <span class="tag">{key}: {value}</span>
                )}
              </For>
            </div>
          </div>
        </Show>

        {/* Resource ID */}
        <div class="detail-section">
          <div class="detail-section-title">Identifiers</div>
          <div class="detail-row">
            <span class="detail-row-label">ID</span>
            <span class="detail-row-value" style={{ "font-size": "11px" }}>
              {props.resource!.id}
            </span>
          </div>
          <div class="detail-row">
            <span class="detail-row-label">Account</span>
            <span>{props.resource!.account_id}</span>
          </div>
        </div>
      </div>
    </Show>
  );
}
