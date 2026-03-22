import dagre from '@dagrejs/dagre';
import type { DmFlowNode, DmFlowEdge } from './types';

const NODE_WIDTH = 260;
const NODE_HEIGHT_BASE = 60;
const PORT_ROW_HEIGHT = 22;

function estimateNodeHeight(node: DmFlowNode): number {
    const portCount = Math.max(node.data.inputs.length, node.data.outputs.length);
    return NODE_HEIGHT_BASE + portCount * PORT_ROW_HEIGHT;
}

export function applyDagreLayout(
    nodes: DmFlowNode[],
    edges: DmFlowEdge[],
): { nodes: DmFlowNode[]; edges: DmFlowEdge[] } {
    const g = new dagre.graphlib.Graph();
    g.setDefaultEdgeLabel(() => ({}));
    g.setGraph({ rankdir: 'LR', nodesep: 60, ranksep: 120 });

    for (const node of nodes) {
        const h = estimateNodeHeight(node);
        g.setNode(node.id, { width: NODE_WIDTH, height: h });
    }
    for (const edge of edges) {
        g.setEdge(edge.source, edge.target);
    }

    dagre.layout(g);

    const layoutNodes = nodes.map((node) => {
        const pos = g.node(node.id);
        const h = estimateNodeHeight(node);
        return {
            ...node,
            position: {
                x: pos.x - NODE_WIDTH / 2,
                y: pos.y - h / 2,
            },
        };
    });

    return { nodes: layoutNodes, edges };
}
