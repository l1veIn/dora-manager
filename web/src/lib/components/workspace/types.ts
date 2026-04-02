// Helper to generate a unique ID
export function generateId(): string {
    return Math.random().toString(36).substring(2, 9);
}

export type WorkspaceWidgetType = "stream" | "input" | "terminal";

export type WorkspaceGridItem = {
    id: string; // unique uuid
    widgetType: WorkspaceWidgetType;
    config: {
        subscribedSources?: string[];
        subscribedInputs?: string[];
        nodeId?: string; 
        [key: string]: any;
    };
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
            widgetType: "stream",
            config: {},
            x: 0, y: 0, w: 8, h: 5
        },
        {
            id: generateId(),
            widgetType: "input",
            config: {},
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
