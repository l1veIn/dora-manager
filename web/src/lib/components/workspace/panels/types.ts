import type { WorkspaceGridItem } from "../types";
import type { PanelConfig, PanelKind } from "../types";

export type PanelApi = {
    close: (panelId: string) => void;
};

export type PanelContext = {
    runId: string;
    snapshots: any[];
    inputValues: Record<string, any>;
    nodes: any[];
    refreshToken: number;
    isRunActive: boolean;
    emitMessage: (message: {
        from: string;
        tag: string;
        payload: any;
        timestamp?: number;
    }) => Promise<void>;
};

export type PanelRendererProps = {
    item: WorkspaceGridItem;
    api: PanelApi;
    context: PanelContext;
    onConfigChange?: () => void;
};

export type PanelSourceMode = "history" | "snapshot" | "external";

export type PanelDefinition = {
    kind: PanelKind;
    title: string;
    dotClass: string;
    sourceMode: PanelSourceMode;
    supportedTags: string[] | "*";
    defaultConfig: PanelConfig;
    component: any;
};
