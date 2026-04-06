// Helper to generate a unique ID
export function generateId(): string {
    return Math.random().toString(36).substring(2, 9);
}

import { getPanelDefinition } from "./panels/registry";

export type PanelKind = "message" | "input" | "chart" | "table" | "video" | "terminal";

export type MessagePanelConfig = {
    nodes?: string[];
    tags?: string[];
    nodeId?: string;
};

export type InputPanelConfig = {
    nodes?: string[];
    tags?: string[];
    nodeId?: string;
    gridCols?: 1 | 2 | 3;
};

export type VideoPanelConfig = {
    mode?: "manual" | "message";
    nodeId?: string;
    selectedSourceId?: string;
    src?: string;
    sourceType?: "auto" | "hls" | "video" | "audio";
    autoplay?: boolean;
    muted?: boolean;
    poster?: string;
    nodes?: string[];
    tags?: string[];
};

export type TerminalPanelConfig = {
    nodeId?: string;
    nodes?: string[];
    tags?: string[];
};

export type PanelConfig = {
    nodes?: string[];
    tags?: string[];
    nodeId?: string;
    [key: string]: any;
};

export type WorkspaceGridItem = {
    id: string; // unique uuid
    widgetType: PanelKind;
    config: PanelConfig;
    x: number;
    y: number;
    w: number;
    h: number;
    min?: { w: number; h: number };
};

// Default layout if none exists
export function getDefaultLayout(): WorkspaceGridItem[] {
    return [
        {
            id: generateId(),
            widgetType: "message",
            config: { ...getPanelDefinition("message").defaultConfig },
            x: 0, y: 0, w: 8, h: 5
        },
        {
            id: generateId(),
            widgetType: "input",
            config: { ...getPanelDefinition("input").defaultConfig },
            x: 8, y: 0, w: 4, h: 5
        }
    ];
}

export function mutateTreeInjectTerminal(layout: WorkspaceGridItem[], targetNodeId: string): WorkspaceGridItem[] {
    // Find existing
    let found = false;
    let newLayout = layout.map(item => {
        if (item.widgetType === "terminal") {
            found = true;
            return { ...item, config: { ...item.config, nodeId: targetNodeId } };
        }
        return item;
    });

    if (found) return newLayout;

    // Inject new at bottom
    let maxY = 0;
    for (let item of layout) {
        maxY = Math.max(maxY, item.y + item.h);
    }

    return [
        ...layout,
        {
            id: generateId(),
            widgetType: "terminal",
            config: { nodeId: targetNodeId },
            x: 0, y: maxY, w: 12, h: 4
        }
    ];
}

export function normalizeWorkspaceLayout(layout: any[]): WorkspaceGridItem[] {
    return layout.map((item) => {
        const widgetType = item?.widgetType === "stream" ? "message" : item?.widgetType;
        const baseConfig = { ...getPanelDefinition(widgetType).defaultConfig };
        const config = { ...baseConfig, ...(item?.config ?? {}) };

        if (widgetType === "message") {
            if (!Array.isArray(config.nodes)) {
                config.nodes = config.subscribedSourceId
                    ? [config.subscribedSourceId]
                    : ["*"];
            }
            if (!Array.isArray(config.tags) || config.tags.length === 0) {
                config.tags = ["*"];
            }
        }

        if (widgetType === "input") {
            if (!Array.isArray(config.nodes)) {
                config.nodes = Array.isArray(config.subscribedInputs) && config.subscribedInputs.length > 0
                    ? config.subscribedInputs
                    : ["*"];
            }
            if (!Array.isArray(config.tags) || config.tags.length === 0) {
                config.tags = ["widgets"];
            }
        }

        delete config.subscribedSourceId;
        delete config.subscribedSources;
        delete config.subscribedInputs;

        return {
            ...item,
            widgetType,
            config,
        };
    });
}
