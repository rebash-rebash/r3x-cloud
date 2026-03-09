import { Show } from "solid-js";
import type { ViewKind, ResourceType } from "../lib/types";
import { resources } from "../stores/scan";
import { analysis } from "../stores/analysis";

interface Props {
  activeView: ViewKind;
  onNavigate: (view: ViewKind) => void;
  onFilterResources: (type?: ResourceType) => void;
  activeTypeFilter: ResourceType | null;
}

export default function Sidebar(props: Props) {
  const resourceCount = (type: string) =>
    resources().filter((r) => r.resource_type === type).length;

  const totalResources = () => resources().length;

  const isTypeActive = (type: ResourceType) =>
    props.activeView === "resources" && props.activeTypeFilter === type;

  return (
    <nav class="sidebar">
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
          class={`sidebar-item ${props.activeView === "resources" && !props.activeTypeFilter ? "active" : ""}`}
          onClick={() => props.onFilterResources()}
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
        <div
          class={`sidebar-item ${props.activeView === "recommendations" ? "active" : ""}`}
          onClick={() => props.onNavigate("recommendations")}
        >
          Findings
          <Show when={analysis() && analysis()!.total_findings > 0}>
            <span class="badge" style={{ background: "var(--color-danger)", color: "#fff" }}>
              {analysis()!.total_findings}
            </span>
          </Show>
        </div>
        <div
          class={`sidebar-item ${props.activeView === "history" ? "active" : ""}`}
          onClick={() => props.onNavigate("history")}
        >
          History
        </div>
        <div
          class={`sidebar-item ${props.activeView === "diff" ? "active" : ""}`}
          onClick={() => props.onNavigate("diff")}
        >
          Scan Diff
        </div>
        <div
          class={`sidebar-item ${props.activeView === "map" ? "active" : ""}`}
          onClick={() => props.onNavigate("map")}
        >
          Infra Map
        </div>
        <div
          class={`sidebar-item ${props.activeView === "accounts" ? "active" : ""}`}
          onClick={() => props.onNavigate("accounts")}
        >
          Accounts
        </div>
      </div>

      {/* Resource types */}
      <Show when={totalResources() > 0}>
        <div class="sidebar-section">
          <div class="sidebar-section-title">By Type</div>
          <Show when={resourceCount("virtual_machine") > 0}>
            <div
              class={`sidebar-item ${isTypeActive("virtual_machine") ? "active" : ""}`}
              onClick={() => props.onFilterResources("virtual_machine")}
            >
              VMs
              <span class="badge">{resourceCount("virtual_machine")}</span>
            </div>
          </Show>
          <Show when={resourceCount("disk") > 0}>
            <div
              class={`sidebar-item ${isTypeActive("disk") ? "active" : ""}`}
              onClick={() => props.onFilterResources("disk")}
            >
              Disks
              <span class="badge">{resourceCount("disk")}</span>
            </div>
          </Show>
          <Show when={resourceCount("snapshot") > 0}>
            <div
              class={`sidebar-item ${isTypeActive("snapshot") ? "active" : ""}`}
              onClick={() => props.onFilterResources("snapshot")}
            >
              Snapshots
              <span class="badge">{resourceCount("snapshot")}</span>
            </div>
          </Show>
          <Show when={resourceCount("elastic_ip") > 0}>
            <div
              class={`sidebar-item ${isTypeActive("elastic_ip") ? "active" : ""}`}
              onClick={() => props.onFilterResources("elastic_ip")}
            >
              Static IPs
              <span class="badge">{resourceCount("elastic_ip")}</span>
            </div>
          </Show>
          <Show when={resourceCount("load_balancer") > 0}>
            <div
              class={`sidebar-item ${isTypeActive("load_balancer") ? "active" : ""}`}
              onClick={() => props.onFilterResources("load_balancer")}
            >
              Load Balancers
              <span class="badge">{resourceCount("load_balancer")}</span>
            </div>
          </Show>
          <Show when={resourceCount("security_group") > 0}>
            <div
              class={`sidebar-item ${isTypeActive("security_group") ? "active" : ""}`}
              onClick={() => props.onFilterResources("security_group")}
            >
              Firewall Rules
              <span class="badge">{resourceCount("security_group")}</span>
            </div>
          </Show>
          <Show when={resourceCount("machine_image") > 0}>
            <div
              class={`sidebar-item ${isTypeActive("machine_image") ? "active" : ""}`}
              onClick={() => props.onFilterResources("machine_image")}
            >
              Images
              <span class="badge">{resourceCount("machine_image")}</span>
            </div>
          </Show>
          <Show when={resourceCount("storage_bucket") > 0}>
            <div
              class={`sidebar-item ${isTypeActive("storage_bucket") ? "active" : ""}`}
              onClick={() => props.onFilterResources("storage_bucket")}
            >
              Buckets
              <span class="badge">{resourceCount("storage_bucket")}</span>
            </div>
          </Show>
          <Show when={resourceCount("serverless_function") > 0}>
            <div
              class={`sidebar-item ${isTypeActive("serverless_function") ? "active" : ""}`}
              onClick={() => props.onFilterResources("serverless_function")}
            >
              Functions
              <span class="badge">{resourceCount("serverless_function")}</span>
            </div>
          </Show>
          <Show when={resourceCount("cloud_sql_instance") > 0}>
            <div
              class={`sidebar-item ${isTypeActive("cloud_sql_instance") ? "active" : ""}`}
              onClick={() => props.onFilterResources("cloud_sql_instance")}
            >
              Cloud SQL
              <span class="badge">{resourceCount("cloud_sql_instance")}</span>
            </div>
          </Show>
          <Show when={resourceCount("cloud_run_service") > 0}>
            <div
              class={`sidebar-item ${isTypeActive("cloud_run_service") ? "active" : ""}`}
              onClick={() => props.onFilterResources("cloud_run_service")}
            >
              Cloud Run
              <span class="badge">{resourceCount("cloud_run_service")}</span>
            </div>
          </Show>
          <Show when={resourceCount("network") > 0}>
            <div
              class={`sidebar-item ${isTypeActive("network") ? "active" : ""}`}
              onClick={() => props.onFilterResources("network")}
            >
              Networks
              <span class="badge">{resourceCount("network")}</span>
            </div>
          </Show>
          <Show when={resourceCount("gke_cluster") > 0}>
            <div
              class={`sidebar-item ${isTypeActive("gke_cluster") ? "active" : ""}`}
              onClick={() => props.onFilterResources("gke_cluster")}
            >
              GKE
              <span class="badge">{resourceCount("gke_cluster")}</span>
            </div>
          </Show>
          <Show when={resourceCount("big_query_dataset") > 0}>
            <div
              class={`sidebar-item ${isTypeActive("big_query_dataset") ? "active" : ""}`}
              onClick={() => props.onFilterResources("big_query_dataset")}
            >
              BigQuery
              <span class="badge">{resourceCount("big_query_dataset")}</span>
            </div>
          </Show>
          <Show when={resourceCount("pub_sub_topic") + resourceCount("pub_sub_subscription") > 0}>
            <div
              class={`sidebar-item ${isTypeActive("pub_sub_topic") ? "active" : ""}`}
              onClick={() => props.onFilterResources("pub_sub_topic")}
            >
              Pub/Sub
              <span class="badge">{resourceCount("pub_sub_topic") + resourceCount("pub_sub_subscription")}</span>
            </div>
          </Show>
          <Show when={resourceCount("spanner_instance") > 0}>
            <div
              class={`sidebar-item ${isTypeActive("spanner_instance") ? "active" : ""}`}
              onClick={() => props.onFilterResources("spanner_instance")}
            >
              Spanner
              <span class="badge">{resourceCount("spanner_instance")}</span>
            </div>
          </Show>
          <Show when={resourceCount("memorystore_instance") > 0}>
            <div
              class={`sidebar-item ${isTypeActive("memorystore_instance") ? "active" : ""}`}
              onClick={() => props.onFilterResources("memorystore_instance")}
            >
              Memorystore
              <span class="badge">{resourceCount("memorystore_instance")}</span>
            </div>
          </Show>
          <Show when={resourceCount("app_engine_version") > 0}>
            <div
              class={`sidebar-item ${isTypeActive("app_engine_version") ? "active" : ""}`}
              onClick={() => props.onFilterResources("app_engine_version")}
            >
              App Engine
              <span class="badge">{resourceCount("app_engine_version")}</span>
            </div>
          </Show>
          <Show when={resourceCount("dataproc_cluster") > 0}>
            <div
              class={`sidebar-item ${isTypeActive("dataproc_cluster") ? "active" : ""}`}
              onClick={() => props.onFilterResources("dataproc_cluster")}
            >
              Dataproc
              <span class="badge">{resourceCount("dataproc_cluster")}</span>
            </div>
          </Show>
          <Show when={resourceCount("artifact_registry_repo") > 0}>
            <div
              class={`sidebar-item ${isTypeActive("artifact_registry_repo") ? "active" : ""}`}
              onClick={() => props.onFilterResources("artifact_registry_repo")}
            >
              Artifact Reg.
              <span class="badge">{resourceCount("artifact_registry_repo")}</span>
            </div>
          </Show>
          <Show when={resourceCount("nat_gateway") > 0}>
            <div
              class={`sidebar-item ${isTypeActive("nat_gateway") ? "active" : ""}`}
              onClick={() => props.onFilterResources("nat_gateway")}
            >
              NAT
              <span class="badge">{resourceCount("nat_gateway")}</span>
            </div>
          </Show>
          <Show when={resourceCount("vpn_tunnel") > 0}>
            <div
              class={`sidebar-item ${isTypeActive("vpn_tunnel") ? "active" : ""}`}
              onClick={() => props.onFilterResources("vpn_tunnel")}
            >
              VPN
              <span class="badge">{resourceCount("vpn_tunnel")}</span>
            </div>
          </Show>
          <Show when={resourceCount("secret_manager_secret") > 0}>
            <div
              class={`sidebar-item ${isTypeActive("secret_manager_secret") ? "active" : ""}`}
              onClick={() => props.onFilterResources("secret_manager_secret")}
            >
              Secrets
              <span class="badge">{resourceCount("secret_manager_secret")}</span>
            </div>
          </Show>
          <Show when={resourceCount("log_sink") > 0}>
            <div
              class={`sidebar-item ${isTypeActive("log_sink") ? "active" : ""}`}
              onClick={() => props.onFilterResources("log_sink")}
            >
              Log Sinks
              <span class="badge">{resourceCount("log_sink")}</span>
            </div>
          </Show>
        </div>
      </Show>

      {/* Settings */}
      <div class="sidebar-section" style={{ "margin-top": "auto" }}>
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
