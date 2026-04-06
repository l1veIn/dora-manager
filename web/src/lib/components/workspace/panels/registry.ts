import MessagePanel from "./message/MessagePanel.svelte";
import InputPanel from "./input/InputPanel.svelte";
import ChartPanel from "./chart/ChartPanel.svelte";
import VideoPanel from "./video/VideoPanel.svelte";
import TerminalPanel from "./terminal/TerminalPanel.svelte";
import type { PanelDefinition } from "./types";
import type { PanelKind } from "../types";

export const panelRegistry: Record<PanelKind, PanelDefinition> = {
    message: {
        kind: "message",
        title: "Message",
        dotClass: "bg-blue-500",
        sourceMode: "history",
        supportedTags: "*",
        defaultConfig: { nodes: ["*"], tags: ["*"] },
        component: MessagePanel,
    },
    input: {
        kind: "input",
        title: "Input",
        dotClass: "bg-orange-500",
        sourceMode: "snapshot",
        supportedTags: ["widgets"],
        defaultConfig: { nodes: ["*"], tags: ["widgets"], gridCols: 2 },
        component: InputPanel,
    },
    chart: {
        kind: "chart",
        title: "Chart",
        dotClass: "bg-emerald-500",
        sourceMode: "snapshot",
        supportedTags: ["chart"],
        defaultConfig: { nodes: ["*"], tags: ["chart"] },
        component: ChartPanel,
    },
    table: {
        kind: "table",
        title: "Table",
        dotClass: "bg-cyan-500",
        sourceMode: "snapshot",
        supportedTags: ["table"],
        defaultConfig: { nodes: ["*"], tags: ["table"] },
        component: MessagePanel,
    },
    video: {
        kind: "video",
        title: "Plyr",
        dotClass: "bg-rose-500",
        sourceMode: "snapshot",
        supportedTags: ["stream"],
        defaultConfig: {
            mode: "manual",
            nodeId: "*",
            selectedSourceId: "",
            src: "",
            sourceType: "hls",
            autoplay: false,
            muted: true,
            poster: "",
            nodes: ["*"],
            tags: ["stream"],
        },
        component: VideoPanel,
    },
    terminal: {
        kind: "terminal",
        title: "Terminal",
        dotClass: "bg-zinc-800 dark:bg-zinc-200",
        sourceMode: "external",
        supportedTags: [],
        defaultConfig: { nodeId: undefined },
        component: TerminalPanel,
    },
};

export function getPanelDefinition(kind: PanelKind): PanelDefinition {
    return panelRegistry[kind] ?? panelRegistry.message;
}
