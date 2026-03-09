import { createSignal, createMemo, createEffect, onMount, onCleanup, For, Show } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import { resources } from "../stores/scan";
import { analysis } from "../stores/analysis";
import { formatCurrency, formatResourceType } from "../lib/format";
import type { CloudResource } from "../lib/types";
import ResourceDetail from "./ResourceDetail";

interface GraphNode {
  id: string;
  resource: CloudResource;
  x: number;
  y: number;
  radius: number;
  color: string;
  hasFindings: boolean;
  savings: number;
}

interface GraphEdge {
  source: string;
  target: string;
}

const TYPE_COLORS: Record<string, string> = {
  virtual_machine: "#58a6ff",
  disk: "#d29922",
  snapshot: "#8b949e",
  elastic_ip: "#3fb950",
  load_balancer: "#bc8cff",
  security_group: "#f0883e",
  machine_image: "#6e7681",
  storage_bucket: "#79c0ff",
  serverless_function: "#d2a8ff",
};

const TYPE_LABELS: Record<string, string> = {
  virtual_machine: "VM",
  disk: "Disk",
  snapshot: "Snap",
  elastic_ip: "IP",
  load_balancer: "LB",
  security_group: "FW",
  machine_image: "Img",
  storage_bucket: "S3",
  serverless_function: "Fn",
};

function shortName(selfLink: string): string {
  if (!selfLink) return "";
  const parts = selfLink.split("/");
  return parts[parts.length - 1] || "";
}

function buildGraph(
  res: CloudResource[],
  findingsByResource: Map<string, number>,
): { nodes: GraphNode[]; edges: GraphEdge[] } {
  const nodes: GraphNode[] = [];
  const edges: GraphEdge[] = [];
  const resourceByName = new Map<string, CloudResource>();
  const selfLinkToId = new Map<string, string>();

  for (const r of res) {
    resourceByName.set(r.name, r);
    const meta = r.metadata as Record<string, unknown>;
    const selfLink = meta?.self_link as string;
    if (selfLink) selfLinkToId.set(selfLink, r.id);
  }

  const edgeSet = new Set<string>();
  const addEdge = (a: string, b: string) => {
    const key = [a, b].sort().join(":");
    if (!edgeSet.has(key)) {
      edgeSet.add(key);
      edges.push({ source: a, target: b });
    }
  };

  // Scale the canvas based on resource count so nodes have room
  const count = res.length;
  const canvasSize = Math.max(800, Math.sqrt(count) * 80);
  const cx = canvasSize / 2;
  const cy = canvasSize / 2;

  for (let i = 0; i < res.length; i++) {
    const r = res[i];
    const cost = r.monthly_cost ?? 0;
    const savings = findingsByResource.get(r.id) ?? 0;
    const radius = Math.max(10, Math.min(30, 10 + Math.sqrt(cost) * 1.5));
    const angle = (i / res.length) * Math.PI * 2;
    const spread = canvasSize * 0.25 + Math.random() * canvasSize * 0.15;

    nodes.push({
      id: r.id,
      resource: r,
      x: cx + Math.cos(angle) * spread,
      y: cy + Math.sin(angle) * spread,
      radius,
      color: TYPE_COLORS[r.resource_type] ?? "#8b949e",
      hasFindings: savings > 0,
      savings,
    });
  }

  // Build edges
  for (const r of res) {
    const meta = r.metadata as Record<string, unknown>;

    if (r.resource_type === "virtual_machine" && meta?.disks) {
      for (const d of meta.disks as Array<Record<string, unknown>>) {
        const source = d.source as string;
        if (source) {
          const disk = resourceByName.get(shortName(source));
          if (disk) addEdge(r.id, disk.id);
        }
      }
    }

    if (r.resource_type === "disk" && Array.isArray(meta?.users)) {
      for (const u of meta.users as string[]) {
        const vm = resourceByName.get(shortName(u));
        if (vm) addEdge(r.id, vm.id);
      }
    }

    if (r.resource_type === "elastic_ip" && Array.isArray(meta?.users)) {
      for (const u of meta.users as string[]) {
        const tid = selfLinkToId.get(u);
        if (tid) addEdge(r.id, tid);
      }
    }

    if (r.resource_type === "load_balancer" && meta?.target) {
      const t = resourceByName.get(meta.target as string);
      if (t) addEdge(r.id, t.id);
    }
  }

  return { nodes, edges };
}

function simulate(nodes: GraphNode[], edges: GraphEdge[], iterations: number) {
  const nodeMap = new Map(nodes.map((n) => [n.id, n]));
  const count = nodes.length;
  const canvasSize = Math.max(800, Math.sqrt(count) * 80);
  const cx = canvasSize / 2;
  const cy = canvasSize / 2;
  const margin = 40;

  for (let iter = 0; iter < iterations; iter++) {
    const alpha = 1 - iter / iterations;
    // Stronger repulsion for more nodes
    const repulsion = (3000 + count * 5) * alpha;
    const attraction = 0.004 * alpha;
    const center = 0.008 * alpha;

    const vx = new Float64Array(count);
    const vy = new Float64Array(count);

    for (let i = 0; i < count; i++) {
      for (let j = i + 1; j < count; j++) {
        const a = nodes[i], b = nodes[j];
        const dx = b.x - a.x || 0.1;
        const dy = b.y - a.y || 0.1;
        const dist = Math.sqrt(dx * dx + dy * dy) || 1;
        const force = repulsion / (dist * dist);
        const fx = (dx / dist) * force;
        const fy = (dy / dist) * force;
        vx[i] -= fx; vy[i] -= fy;
        vx[j] += fx; vy[j] += fy;
      }
    }

    for (const edge of edges) {
      const ai = nodes.findIndex((n) => n.id === edge.source);
      const bi = nodes.findIndex((n) => n.id === edge.target);
      if (ai < 0 || bi < 0) continue;
      const a = nodes[ai], b = nodes[bi];
      const dx = b.x - a.x;
      const dy = b.y - a.y;
      const dist = Math.sqrt(dx * dx + dy * dy) || 1;
      const force = dist * attraction;
      vx[ai] += (dx / dist) * force;
      vy[ai] += (dy / dist) * force;
      vx[bi] -= (dx / dist) * force;
      vy[bi] -= (dy / dist) * force;
    }

    for (let i = 0; i < count; i++) {
      const n = nodes[i];
      vx[i] += (cx - n.x) * center;
      vy[i] += (cy - n.y) * center;
      n.x += vx[i] * 0.6;
      n.y += vy[i] * 0.6;
      n.x = Math.max(margin, Math.min(canvasSize - margin, n.x));
      n.y = Math.max(margin, Math.min(canvasSize - margin, n.y));
    }
  }
}

export default function InfraMap() {
  const [hoveredId, setHoveredId] = createSignal<string | null>(null);
  const [detailResource, setDetailResource] = createSignal<CloudResource | null>(null);
  const [tooltipPos, setTooltipPos] = createSignal({ x: 0, y: 0 });
  const [viewBox, setViewBox] = createSignal({ x: 0, y: 0, w: 800, h: 800 });

  // Mutable node positions for dragging
  const [nodePositions, setNodePositions] = createStore<Record<string, { x: number; y: number }>>({});

  let svgRef: SVGSVGElement | undefined;
  let containerRef: HTMLDivElement | undefined;
  let isPanning = false;
  let panStart = { x: 0, y: 0 };
  let vbStart = { x: 0, y: 0 };
  let dragNodeId: string | null = null;
  let didDrag = false;

  const findingSavings = createMemo(() => {
    const data = analysis();
    const map = new Map<string, number>();
    if (data) {
      for (const f of data.findings) {
        map.set(f.resource_id, (map.get(f.resource_id) ?? 0) + f.estimated_monthly_savings);
      }
    }
    return map;
  });

  const graphData = createMemo(() => {
    const res = resources();
    if (res.length === 0) return { nodes: [], edges: [] };
    const g = buildGraph(res, findingSavings());
    simulate(g.nodes, g.edges, 120);

    // Scale viewBox to fit
    const canvasSize = Math.max(800, Math.sqrt(res.length) * 80);
    setViewBox({ x: 0, y: 0, w: canvasSize, h: canvasSize });

    return g;
  });

  // Sync initial positions from layout into mutable store
  createEffect(() => {
    const g = graphData();
    const pos: Record<string, { x: number; y: number }> = {};
    for (const n of g.nodes) {
      pos[n.id] = { x: n.x, y: n.y };
    }
    setNodePositions(reconcile(pos));
  });

  const nodeById = createMemo(() => {
    const map = new Map<string, GraphNode>();
    for (const n of graphData().nodes) map.set(n.id, n);
    return map;
  });

  const totalCost = createMemo(() =>
    graphData().nodes.reduce((sum, n) => sum + (n.resource.monthly_cost ?? 0), 0)
  );
  const totalSavings = createMemo(() =>
    graphData().nodes.reduce((sum, n) => sum + n.savings, 0)
  );
  const orphanCount = createMemo(() => {
    const connected = new Set<string>();
    for (const e of graphData().edges) {
      connected.add(e.source);
      connected.add(e.target);
    }
    return graphData().nodes.filter((n) => !connected.has(n.id)).length;
  });

  // Convert screen coords to SVG coords
  const screenToSvg = (clientX: number, clientY: number) => {
    if (!svgRef) return { x: 0, y: 0 };
    const rect = svgRef.getBoundingClientRect();
    const vb = viewBox();
    return {
      x: ((clientX - rect.left) / rect.width) * vb.w + vb.x,
      y: ((clientY - rect.top) / rect.height) * vb.h + vb.y,
    };
  };

  // Zoom
  const handleWheel = (e: WheelEvent) => {
    e.preventDefault();
    const vb = viewBox();
    const scale = e.deltaY > 0 ? 1.1 : 0.9;
    if (svgRef) {
      const pt = screenToSvg(e.clientX, e.clientY);
      setViewBox({
        x: pt.x - (pt.x - vb.x) * scale,
        y: pt.y - (pt.y - vb.y) * scale,
        w: vb.w * scale,
        h: vb.h * scale,
      });
    }
  };

  // Mouse down: either drag a node or pan
  const handleMouseDown = (e: MouseEvent) => {
    if (e.button !== 0) return;
    const nodeEl = (e.target as SVGElement).closest(".map-node");
    if (nodeEl) {
      const id = nodeEl.getAttribute("data-id");
      if (id) {
        dragNodeId = id;
        didDrag = false;
        e.preventDefault();
        return;
      }
    }
    isPanning = true;
    panStart = { x: e.clientX, y: e.clientY };
    const vb = viewBox();
    vbStart = { x: vb.x, y: vb.y };
  };

  const handleMouseMove = (e: MouseEvent) => {
    // Tooltip position
    if (containerRef) {
      const rect = containerRef.getBoundingClientRect();
      setTooltipPos({ x: e.clientX - rect.left + 16, y: e.clientY - rect.top - 10 });
    }

    // Node dragging
    if (dragNodeId) {
      didDrag = true;
      const pt = screenToSvg(e.clientX, e.clientY);
      setNodePositions(dragNodeId, { x: pt.x, y: pt.y });
      return;
    }

    // Panning
    if (!isPanning || !svgRef) return;
    const rect = svgRef.getBoundingClientRect();
    const vb = viewBox();
    const dx = ((e.clientX - panStart.x) / rect.width) * vb.w;
    const dy = ((e.clientY - panStart.y) / rect.height) * vb.h;
    setViewBox({ ...vb, x: vbStart.x - dx, y: vbStart.y - dy });
  };

  const handleMouseUp = () => {
    dragNodeId = null;
    isPanning = false;
  };

  const handleNodeClick = (resource: CloudResource) => {
    // Only open detail if we didn't drag
    if (!didDrag) {
      setDetailResource(resource);
    }
    didDrag = false;
  };

  onMount(() => {
    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
  });

  onCleanup(() => {
    document.removeEventListener("mousemove", handleMouseMove);
    document.removeEventListener("mouseup", handleMouseUp);
  });

  const getPos = (id: string) => nodePositions[id] ?? { x: 0, y: 0 };
  const hoveredNode = () => {
    const id = hoveredId();
    return id ? nodeById().get(id) ?? null : null;
  };

  return (
    <div style={{ display: "flex", "flex-direction": "column", height: "100%" }}>
      {/* Header */}
      <div style={{
        display: "flex", "align-items": "center", "justify-content": "space-between",
        "margin-bottom": "10px", "flex-shrink": "0",
      }}>
        <h2 style={{ "font-size": "16px", margin: "0" }}>Infrastructure Map</h2>
        <button class="btn btn-sm" onClick={() => {
          const count = graphData().nodes.length;
          const sz = Math.max(800, Math.sqrt(count) * 80);
          setViewBox({ x: 0, y: 0, w: sz, h: sz });
        }}>
          Reset View
        </button>
      </div>

      {/* Stats */}
      <div style={{
        display: "flex", "flex-wrap": "wrap", gap: "16px",
        "padding-bottom": "8px", "margin-bottom": "8px",
        "border-bottom": "1px solid var(--border-secondary)",
        "font-size": "12px", color: "var(--text-muted)", "flex-shrink": "0",
      }}>
        <span><strong style={{ color: "var(--text-primary)" }}>{graphData().nodes.length}</strong> resources</span>
        <span><strong style={{ color: "var(--text-primary)" }}>{graphData().edges.length}</strong> connections</span>
        <span><strong style={{ color: "var(--text-primary)" }}>{orphanCount()}</strong> orphaned</span>
        <span>Cost <strong style={{ color: "var(--text-primary)" }}>{formatCurrency(totalCost())}</strong>/mo</span>
        <Show when={totalSavings() > 0}>
          <span style={{ color: "var(--color-success)" }}>
            Save <strong>{formatCurrency(totalSavings())}</strong>/mo
          </span>
        </Show>
      </div>

      {/* Legend */}
      <div style={{
        display: "flex", "flex-wrap": "wrap", gap: "10px",
        "margin-bottom": "8px", "font-size": "11px",
        color: "var(--text-muted)", "flex-shrink": "0",
      }}>
        <For each={Object.entries(TYPE_COLORS)}>
          {([type, color]) => (
            <span style={{ display: "flex", "align-items": "center", gap: "4px" }}>
              <span style={{
                width: "8px", height: "8px", "border-radius": "50%",
                background: color, display: "inline-block",
              }} />
              {TYPE_LABELS[type] || type}
            </span>
          )}
        </For>
        <span style={{ display: "flex", "align-items": "center", gap: "4px" }}>
          <span style={{
            width: "8px", height: "8px", "border-radius": "50%",
            border: "2px dashed #f85149", display: "inline-block",
          }} />
          Finding
        </span>
      </div>

      {/* Canvas */}
      <Show
        when={resources().length > 0}
        fallback={
          <div class="empty-state" style={{ flex: "1" }}>
            <div class="empty-state-title">No resources to map</div>
            <p style={{ color: "var(--text-muted)", "font-size": "13px" }}>
              Run a scan to visualize your infrastructure.
            </p>
          </div>
        }
      >
        <div ref={containerRef} style={{
          flex: "1", position: "relative", overflow: "hidden",
          "border-radius": "8px", background: "var(--bg-secondary)",
          border: "1px solid var(--border-primary)",
        }}>
          <svg
            ref={svgRef}
            class="infra-map-svg"
            viewBox={`${viewBox().x} ${viewBox().y} ${viewBox().w} ${viewBox().h}`}
            onWheel={handleWheel}
            onMouseDown={handleMouseDown}
          >
            {/* Grid */}
            <defs>
              <pattern id="grid" width="40" height="40" patternUnits="userSpaceOnUse">
                <path d="M 40 0 L 0 0 0 40" fill="none" stroke="var(--border-secondary)" stroke-width="0.5" />
              </pattern>
            </defs>
            <rect x={viewBox().x} y={viewBox().y} width={viewBox().w} height={viewBox().h} fill="url(#grid)" />

            {/* Edges */}
            <For each={graphData().edges}>
              {(edge) => {
                const sp = () => getPos(edge.source);
                const tp = () => getPos(edge.target);
                return (
                  <line
                    x1={sp().x} y1={sp().y} x2={tp().x} y2={tp().y}
                    stroke="var(--text-muted)" stroke-width="0.8" opacity="0.25"
                  />
                );
              }}
            </For>

            {/* Nodes */}
            <For each={graphData().nodes}>
              {(node) => {
                const pos = () => getPos(node.id);
                const isHovered = () => hoveredId() === node.id;
                return (
                  <g
                    class="map-node"
                    data-id={node.id}
                    style={{ cursor: "grab" }}
                    onMouseEnter={() => setHoveredId(node.id)}
                    onMouseLeave={() => setHoveredId(null)}
                    onClick={() => handleNodeClick(node.resource)}
                  >
                    {/* Finding ring */}
                    <Show when={node.hasFindings}>
                      <circle
                        cx={pos().x} cy={pos().y} r={node.radius + 4}
                        fill="none" stroke="#f85149" stroke-width="2"
                        stroke-dasharray="4,3" opacity="0.7"
                      />
                    </Show>

                    {/* Shadow */}
                    <circle
                      cx={pos().x + 1} cy={pos().y + 2} r={node.radius}
                      fill="rgba(0,0,0,0.12)"
                    />

                    {/* Main circle */}
                    <circle
                      cx={pos().x} cy={pos().y} r={node.radius}
                      fill={node.color}
                      opacity={isHovered() ? "1" : "0.85"}
                      stroke={isHovered() ? "#fff" : "rgba(255,255,255,0.1)"}
                      stroke-width={isHovered() ? "2" : "0.5"}
                    />

                    {/* Type label */}
                    <text
                      x={pos().x} y={pos().y + 1}
                      text-anchor="middle" dominant-baseline="central"
                      fill="#fff" font-size={node.radius > 16 ? "10" : "8"}
                      font-weight="700" pointer-events="none"
                    >
                      {TYPE_LABELS[node.resource.resource_type] ?? "?"}
                    </text>
                  </g>
                );
              }}
            </For>
          </svg>

          {/* Tooltip */}
          <Show when={hoveredNode()}>
            <div
              class="map-tooltip"
              style={{
                left: `${tooltipPos().x}px`,
                top: `${tooltipPos().y}px`,
              }}
            >
              <div class="map-tooltip-title">{hoveredNode()!.resource.name}</div>
              <div class="map-tooltip-row">
                <span class="map-tooltip-label">Type</span>
                <span class="map-tooltip-value">{formatResourceType(hoveredNode()!.resource.resource_type)}</span>
              </div>
              <div class="map-tooltip-row">
                <span class="map-tooltip-label">Region</span>
                <span class="map-tooltip-value">{hoveredNode()!.resource.region}</span>
              </div>
              <div class="map-tooltip-row">
                <span class="map-tooltip-label">Status</span>
                <span class="map-tooltip-value">{hoveredNode()!.resource.status}</span>
              </div>
              <Show when={hoveredNode()!.resource.monthly_cost}>
                <div class="map-tooltip-row">
                  <span class="map-tooltip-label">Cost</span>
                  <span class="map-tooltip-value">{formatCurrency(hoveredNode()!.resource.monthly_cost)}/mo</span>
                </div>
              </Show>
              <Show when={hoveredNode()!.savings > 0}>
                <div class="map-tooltip-row">
                  <span class="map-tooltip-label" style={{ color: "var(--color-success)" }}>Savings</span>
                  <span class="map-tooltip-value" style={{ color: "var(--color-success)" }}>{formatCurrency(hoveredNode()!.savings)}/mo</span>
                </div>
              </Show>
            </div>
          </Show>

          {/* Hint */}
          <div style={{
            position: "absolute", bottom: "8px", right: "12px",
            "font-size": "10px", color: "var(--text-muted)", opacity: "0.5",
          }}>
            Scroll to zoom · Drag nodes to move · Click for details
          </div>
        </div>
      </Show>

      <ResourceDetail
        resource={detailResource()}
        onClose={() => setDetailResource(null)}
      />
    </div>
  );
}
