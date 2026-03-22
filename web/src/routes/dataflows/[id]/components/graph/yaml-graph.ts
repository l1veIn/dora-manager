import YAML from 'yaml';
import type { DmFlowNode, DmFlowEdge, DmNodeData, ViewJson } from './types';
import { classifyInput } from './types';
import { applyDagreLayout } from './auto-layout';

/**
 * Generate a unique YAML ID for a new node.
 */
export function generateNodeId(
    nodeType: string,
    existingIds: Set<string>,
): string {
    // Use the node type as the base ID (e.g., "dora-keyboard")
    const base = nodeType.split('/').pop() || nodeType;
    if (!existingIds.has(base)) return base;
    let i = 1;
    while (existingIds.has(`${base}-${i}`)) i++;
    return `${base}-${i}`;
}

/**
 * Create a DmFlowNode from palette drop data.
 */
export function createNodeFromPalette(
    paletteData: {
        nodeId: string;
        nodeName: string;
        inputs: string[];
        outputs: string[];
        dynamicPorts?: boolean;
    },
    position: { x: number; y: number },
    existingIds: Set<string>,
): DmFlowNode {
    const id = generateNodeId(paletteData.nodeId, existingIds);
    return {
        id,
        type: 'dmNode',
        position,
        data: {
            label: id,
            nodeType: paletteData.nodeId,
            inputs: paletteData.inputs,
            outputs: paletteData.outputs,
        },
    };
}

interface YamlNode {
    id: string;
    node?: string;
    path?: string;
    inputs?: Record<string, string>;
    outputs?: string[];
    config?: Record<string, unknown>;
    env?: Record<string, unknown>;
    widgets?: Record<string, unknown>;
}

/**
 * Parse a dataflow YAML string + view.json into SvelteFlow nodes and edges.
 */
export function yamlToGraph(
    yamlStr: string,
    viewJson: ViewJson = {},
): { nodes: DmFlowNode[]; edges: DmFlowEdge[] } {
    let parsed: { nodes?: YamlNode[] };
    try {
        parsed = YAML.parse(yamlStr) || {};
    } catch {
        return { nodes: [], edges: [] };
    }

    const yamlNodes = parsed.nodes || [];
    const nodes: DmFlowNode[] = [];
    const edges: DmFlowEdge[] = [];
    const virtualNodeIds = new Set<string>();

    // First pass: create real nodes
    for (const yn of yamlNodes) {
        const pos = viewJson.nodes?.[yn.id];
        const inputKeys = yn.inputs ? Object.keys(yn.inputs) : [];
        const outputKeys = yn.outputs || [];

        nodes.push({
            id: yn.id,
            type: 'dmNode',
            position: pos ? { x: pos.x, y: pos.y } : { x: 0, y: 0 },
            data: {
                label: yn.id,
                nodeType: yn.node || yn.path || 'unknown',
                inputs: inputKeys,
                outputs: outputKeys,
            },
        });
    }

    // Second pass: derive edges + virtual nodes
    for (const yn of yamlNodes) {
        if (!yn.inputs) continue;
        for (const [inputPort, sourceStr] of Object.entries(yn.inputs)) {
            const src = classifyInput(sourceStr);

            if (src.type === 'node') {
                edges.push({
                    id: `e-${src.sourceId}-${src.outputPort}-${yn.id}-${inputPort}`,
                    source: src.sourceId,
                    target: yn.id,
                    sourceHandle: `out-${src.outputPort}`,
                    targetHandle: `in-${inputPort}`,
                });
            } else if (src.type === 'dora') {
                const virtualId = `__virtual_${src.raw.replace(/\//g, '_')}`;
                if (!virtualNodeIds.has(virtualId)) {
                    virtualNodeIds.add(virtualId);
                    const pos = viewJson.nodes?.[virtualId];
                    const parts = src.raw.split('/');
                    const label =
                        parts.length >= 4
                            ? `Timer ${parts[3]}ms`
                            : src.raw;

                    nodes.push({
                        id: virtualId,
                        type: 'dmNode',
                        position: pos
                            ? { x: pos.x, y: pos.y }
                            : { x: 0, y: 0 },
                        data: {
                            label,
                            nodeType: src.raw,
                            inputs: [],
                            outputs: ['tick'],
                            isVirtual: true,
                            virtualKind: 'timer',
                        },
                    });
                }
                edges.push({
                    id: `e-${virtualId}-tick-${yn.id}-${inputPort}`,
                    source: virtualId,
                    target: yn.id,
                    sourceHandle: 'out-tick',
                    targetHandle: `in-${inputPort}`,
                });
            } else if (src.type === 'panel') {
                const virtualId = '__virtual_panel';
                if (!virtualNodeIds.has(virtualId)) {
                    virtualNodeIds.add(virtualId);
                    const pos = viewJson.nodes?.[virtualId];
                    nodes.push({
                        id: virtualId,
                        type: 'dmNode',
                        position: pos
                            ? { x: pos.x, y: pos.y }
                            : { x: 0, y: 0 },
                        data: {
                            label: 'Panel Inputs',
                            nodeType: 'panel',
                            inputs: [],
                            outputs: [],
                            isVirtual: true,
                            virtualKind: 'panel',
                        },
                    });
                }
                // Add the widget output to the virtual panel node's port list
                const panelNode = nodes.find((n) => n.id === virtualId);
                if (
                    panelNode &&
                    !panelNode.data.outputs.includes(src.widgetId)
                ) {
                    panelNode.data.outputs = [
                        ...panelNode.data.outputs,
                        src.widgetId,
                    ];
                }
                edges.push({
                    id: `e-panel-${src.widgetId}-${yn.id}-${inputPort}`,
                    source: virtualId,
                    target: yn.id,
                    sourceHandle: `out-${src.widgetId}`,
                    targetHandle: `in-${inputPort}`,
                });
            }
        }
    }

    // Apply auto-layout if any node lacks a stored position
    const needsLayout = nodes.some(
        (n) =>
            n.position.x === 0 &&
            n.position.y === 0 &&
            !viewJson.nodes?.[n.id],
    );
    if (needsLayout) {
        return applyDagreLayout(nodes, edges);
    }

    return { nodes, edges };
}

/**
 * Build a view.json from the current canvas state.
 */
export function buildViewJson(
    nodes: DmFlowNode[],
    viewport?: { x: number; y: number; zoom: number },
): ViewJson {
    const view: ViewJson = {};
    if (viewport) view.viewport = viewport;
    view.nodes = {};
    for (const n of nodes) {
        view.nodes[n.id] = { x: n.position.x, y: n.position.y };
    }
    return view;
}

/**
 * Convert a SvelteFlow edge back to the YAML input value string.
 * e.g. "dm-microphone/audio" or "dora/timer/millis/2000" or "panel/device_id"
 */
function resolveEdgeToInputValue(
    edge: DmFlowEdge,
    nodes: DmFlowNode[],
): string | null {
    const sourceNode = nodes.find((n) => n.id === edge.source);
    if (!sourceNode) return null;

    if (sourceNode.data.isVirtual) {
        if (sourceNode.data.virtualKind === 'timer') {
            return sourceNode.data.nodeType as string;
        }
        if (sourceNode.data.virtualKind === 'panel') {
            const port = (edge.sourceHandle || '').replace('out-', '');
            return `panel/${port}`;
        }
        return null;
    }

    const outputPort = (edge.sourceHandle || '').replace('out-', '');
    return `${sourceNode.id}/${outputPort}`;
}

/**
 * Serialize the current graph state back to a valid dataflow YAML string.
 * Preserves config/env/widgets from the original YAML.
 */
export function graphToYaml(
    nodes: DmFlowNode[],
    edges: DmFlowEdge[],
    originalYamlStr?: string,
): string {
    // Parse original YAML to preserve non-graph fields
    let originalParsed: any = {};
    const originalNodeMap = new Map<string, any>();
    if (originalYamlStr) {
        try {
            originalParsed = YAML.parse(originalYamlStr) || {};
        } catch {
            /* ignore */
        }
        for (const n of originalParsed.nodes || []) {
            originalNodeMap.set(n.id, n);
        }
    }

    // Build edge index: target node ID → [{inputPort, sourceStr}]
    const edgeIndex = new Map<string, { port: string; source: string }[]>();
    for (const edge of edges) {
        const sourceStr = resolveEdgeToInputValue(edge, nodes);
        if (!sourceStr) continue;
        const inputPort = (edge.targetHandle || '').replace('in-', '');
        const arr = edgeIndex.get(edge.target) || [];
        arr.push({ port: inputPort, source: sourceStr });
        edgeIndex.set(edge.target, arr);
    }

    // Build YAML nodes (skip virtual nodes)
    const yamlNodes: any[] = [];
    for (const node of nodes) {
        if (node.data.isVirtual) continue;

        const orig = originalNodeMap.get(node.id);
        const inputs: Record<string, string> = {};
        for (const { port, source } of edgeIndex.get(node.id) || []) {
            inputs[port] = source;
        }

        const entry: any = { id: node.id };

        // Use 'path' if original had 'path', otherwise use 'node'
        if (orig?.path) {
            entry.path = orig.path;
        } else {
            entry.node = node.data.nodeType;
        }

        if (Object.keys(inputs).length > 0) entry.inputs = inputs;

        const outputs = node.data.outputs as string[];
        if (outputs.length > 0) entry.outputs = outputs;

        // Preserve original config/env/widgets
        if (orig?.config) entry.config = orig.config;
        if (orig?.env) entry.env = orig.env;
        if (orig?.widgets) entry.widgets = orig.widgets;

        yamlNodes.push(entry);
    }

    // Preserve top-level fields from original (e.g. communication layer)
    const result: any = {};
    for (const [key, value] of Object.entries(originalParsed)) {
        if (key !== 'nodes') result[key] = value;
    }
    result.nodes = yamlNodes;

    return YAML.stringify(result);
}
