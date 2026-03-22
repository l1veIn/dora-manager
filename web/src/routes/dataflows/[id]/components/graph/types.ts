import type { Node, Edge } from '@xyflow/svelte';

// Data payload attached to each SvelteFlow node
export interface DmNodeData {
    [key: string]: unknown;
    label: string;
    nodeType: string;
    inputs: string[];
    outputs: string[];
    isVirtual?: boolean;
    virtualKind?: 'timer' | 'panel';
}

export interface ViewNodePosition {
    x: number;
    y: number;
}

export interface ViewJson {
    viewport?: { x: number; y: number; zoom: number };
    nodes?: Record<string, ViewNodePosition>;
}

export type DmFlowNode = Node<DmNodeData, 'dmNode'>;
export type DmFlowEdge = Edge;

// Classify an input source string
export type InputSource =
    | { type: 'node'; sourceId: string; outputPort: string }
    | { type: 'dora'; raw: string }
    | { type: 'panel'; widgetId: string };

export function classifyInput(value: string): InputSource {
    if (value.startsWith('dora/'))
        return { type: 'dora', raw: value };
    if (value.startsWith('panel/'))
        return { type: 'panel', widgetId: value.split('/')[1] };
    const slashIdx = value.indexOf('/');
    if (slashIdx > 0) {
        return {
            type: 'node',
            sourceId: value.substring(0, slashIdx),
            outputPort: value.substring(slashIdx + 1),
        };
    }
    return { type: 'dora', raw: value };
}
