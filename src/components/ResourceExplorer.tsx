import { For, Show, createSignal, createMemo } from "solid-js";
import { resources } from "../stores/scan";
import {
  formatCurrency,
  formatDate,
  formatResourceType,
  statusColor,
} from "../lib/format";
import type { CloudResource, ResourceType } from "../lib/types";
import ResourceDetail from "./ResourceDetail";

interface Props {
  typeFilter: ResourceType | null;
  onClearFilter: () => void;
}

type SortField = "name" | "status" | "region" | "resource_type" | "monthly_cost";
type SortDir = "asc" | "desc";

export default function ResourceExplorer(props: Props) {
  const [sortField, setSortField] = createSignal<SortField>("name");
  const [sortDir, setSortDir] = createSignal<SortDir>("asc");
  const [selectedIdx, setSelectedIdx] = createSignal(-1);
  const [detailResource, setDetailResource] = createSignal<CloudResource | null>(null);
  const [searchQuery, setSearchQuery] = createSignal("");

  const filteredResources = createMemo(() => {
    let items = resources();

    // Filter by resource type
    const tf = props.typeFilter;
    if (tf) {
      items = items.filter((r) => r.resource_type === tf);
    }

    // Filter by search query
    const q = searchQuery().toLowerCase();
    if (q) {
      items = items.filter(
        (r) =>
          r.name.toLowerCase().includes(q) ||
          r.region.toLowerCase().includes(q) ||
          r.status.toLowerCase().includes(q) ||
          r.resource_type.toLowerCase().includes(q) ||
          Object.values(r.tags).some((v) => v.toLowerCase().includes(q)),
      );
    }

    // Sort
    const field = sortField();
    const dir = sortDir();
    return [...items].sort((a, b) => {
      let cmp = 0;
      const aVal = a[field as keyof CloudResource];
      const bVal = b[field as keyof CloudResource];

      if (typeof aVal === "string" && typeof bVal === "string") {
        cmp = aVal.localeCompare(bVal);
      } else if (typeof aVal === "number" && typeof bVal === "number") {
        cmp = aVal - bVal;
      } else {
        cmp = String(aVal ?? "").localeCompare(String(bVal ?? ""));
      }

      return dir === "asc" ? cmp : -cmp;
    });
  });

  const toggleSort = (field: SortField) => {
    if (sortField() === field) {
      setSortDir((d) => (d === "asc" ? "desc" : "asc"));
    } else {
      setSortField(field);
      setSortDir("asc");
    }
  };

  const sortIndicator = (field: SortField) => {
    if (sortField() !== field) return "";
    return sortDir() === "asc" ? " ^" : " v";
  };

  const handleRowClick = (resource: CloudResource, idx: number) => {
    setSelectedIdx(idx);
    setDetailResource(resource);
  };

  return (
    <div>
      <div
        style={{
          display: "flex",
          "align-items": "center",
          "justify-content": "space-between",
          "margin-bottom": "12px",
        }}
      >
        <div style={{ display: "flex", "align-items": "center", gap: "8px" }}>
          <h2 style={{ "font-size": "18px" }}>Resources</h2>
          <Show when={props.typeFilter}>
            <span
              class="status-badge"
              style={{
                color: "var(--color-accent)",
                background: "var(--bg-active)",
                cursor: "pointer",
                "font-size": "11px",
              }}
              onClick={props.onClearFilter}
            >
              {formatResourceType(props.typeFilter!)} &times;
            </span>
          </Show>
        </div>
        <div style={{ display: "flex", "align-items": "center", gap: "12px" }}>
          <input
            id="search-input"
            class="header-search"
            type="text"
            placeholder="Search resources... (/)"
            value={searchQuery()}
            onInput={(e) => setSearchQuery(e.currentTarget.value)}
          />
          <span style={{ "font-size": "12px", color: "var(--text-muted)", "white-space": "nowrap" }}>
            {filteredResources().length} resource
            {filteredResources().length !== 1 ? "s" : ""}
          </span>
        </div>
      </div>

      <Show
        when={resources().length > 0}
        fallback={
          <div class="empty-state">
            <div class="empty-state-title">No resources</div>
            <p>Run a scan to discover cloud resources.</p>
          </div>
        }
      >
        <div class="resource-table-wrapper">
          <table class="resource-table">
            <thead>
              <tr>
                <th onClick={() => toggleSort("name")}>
                  Name{sortIndicator("name")}
                </th>
                <th onClick={() => toggleSort("resource_type")}>
                  Type{sortIndicator("resource_type")}
                </th>
                <th onClick={() => toggleSort("status")}>
                  Status{sortIndicator("status")}
                </th>
                <th onClick={() => toggleSort("region")}>
                  Region{sortIndicator("region")}
                </th>
                <th onClick={() => toggleSort("monthly_cost")}>
                  Cost/mo{sortIndicator("monthly_cost")}
                </th>
                <th>Machine Type</th>
                <th>Created</th>
              </tr>
            </thead>
            <tbody>
              <For each={filteredResources()}>
                {(resource, idx) => (
                  <tr
                    class={selectedIdx() === idx() ? "selected" : ""}
                    onClick={() => handleRowClick(resource, idx())}
                    style={{ cursor: "pointer" }}
                  >
                    <td title={resource.name}>{resource.name}</td>
                    <td>{formatResourceType(resource.resource_type)}</td>
                    <td>
                      <span
                        class="status-badge"
                        style={{
                          color: statusColor(resource.status),
                          background: `${statusColor(resource.status)}20`,
                        }}
                      >
                        {resource.status}
                      </span>
                    </td>
                    <td>{resource.region}</td>
                    <td>{formatCurrency(resource.monthly_cost)}</td>
                    <td>
                      {(resource.metadata as Record<string, unknown>)
                        ?.machine_type as string || "-"}
                    </td>
                    <td>{formatDate(resource.created_at)}</td>
                  </tr>
                )}
              </For>
            </tbody>
          </table>
        </div>
      </Show>

      <ResourceDetail
        resource={detailResource()}
        onClose={() => setDetailResource(null)}
      />
    </div>
  );
}
