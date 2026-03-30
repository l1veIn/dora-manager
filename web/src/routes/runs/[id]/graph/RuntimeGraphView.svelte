<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import {
        SvelteFlow,
        Controls,
        MiniMap,
        Background,
        BackgroundVariant,
    } from "@xyflow/svelte";
    import "@xyflow/svelte/dist/style.css";
    import { mode } from "mode-watcher";
    import RuntimeNode from "./RuntimeNode.svelte";
    import NodeInspector from "./NodeInspector.svelte";
    import { yamlToGraph } from "../../../dataflows/[id]/components/graph/yaml-graph";
    import type { ViewJson, DmFlowNode, DmFlowEdge } from "../../../dataflows/[id]/components/graph/types";

    let {
        runId,
        yamlContent,
        viewJson,
    }: {
        runId: string;
        yamlContent: string;
        viewJson: ViewJson;
    } = $props();

    const nodeTypes: any = { dmNode: RuntimeNode };

    let nodes = $state<DmFlowNode[]>([]);
    let edges = $state<DmFlowEdge[]>([]);

    let selectedNode = $state<DmFlowNode | null>(null);
    let logsMap = $state<Record<string, string[]>>({});
    let ioMap = $state<Record<string, string[]>>({});
    let metricsMap = $state<Map<string, any>>(new Map());

    let colorMode = $derived(
        mode.current === "dark" ? "dark" : ("light" as "dark" | "light"),
    );

    let ws: WebSocket | null = null;
    let globalStatus = $state("unknown");

    function connectWs() {
        const proto = location.protocol === "https:" ? "wss:" : "ws:";
        const url = `${proto}//${location.host}/api/runs/${runId}/ws`;
        
        ws = new WebSocket(url);

        ws.onopen = () => {
            console.log("[run_ws] Connected");
        };

        ws.onmessage = (event) => {
            try {
                const msg = JSON.parse(event.data);
                handleWsMessage(msg);
            } catch (e) {
                console.error("[run_ws] parse error", e, event.data);
            }
        };

        ws.onclose = () => {
            console.log("[run_ws] Closed");
        };
    }

    function handleWsMessage(msg: any) {
        if (msg.type === "ping") return;
        
        if (msg.type === "status") {
            globalStatus = msg.status;
            // Update all nodes basic status if dataflow is stopped or failed
            nodes = nodes.map((n: DmFlowNode) => ({
                ...n,
                data: { ...n.data, status: globalStatus.toLowerCase() }
            }));
            
            // if we are running, make edges animate!
            edges = edges.map((e: DmFlowEdge) => ({
                ...e, 
                animated: globalStatus.toLowerCase() === "running",
                style: globalStatus.toLowerCase() === "running" ? "stroke: #3b82f6; stroke-width: 2" : "",
            }));
        } 
        else if (msg.type === "metrics") {
            const newMetrics = new Map(metricsMap);
            for (const item of msg.data) {
                newMetrics.set(item.id, item);
            }
            metricsMap = newMetrics;

            // Apply metrics to nodes
            nodes = nodes.map((n) => {
                if (metricsMap.has(n.id)) {
                    const m = metricsMap.get(n.id);
                    return {
                        ...n,
                        data: {
                            ...n.data,
                            cpu: m.cpu,
                            memory: m.memory,
                            status: "running"
                        }
                    };
                }
                return n;
            });
        } 
        else if (msg.type === "logs" || msg.type === "io") {
            const nodeId = msg.nodeId;
            // Flash node log indicator
            nodes = nodes.map((n) => {
                if (n.id === nodeId) {
                    return {
                        ...n,
                        data: {
                            ...n.data,
                            hasLogs: true
                        }
                    };
                }
                return n;
            });
            // Store lines
            if (msg.type === "logs") {
                if (!logsMap[nodeId]) logsMap[nodeId] = [];
                // Only keep last 1000 lines
                logsMap[nodeId] = [...logsMap[nodeId], ...msg.lines].slice(-1000);
            } else if (msg.type === "io") {
                if (!ioMap[nodeId]) ioMap[nodeId] = [];
                ioMap[nodeId] = [...ioMap[nodeId], ...msg.lines].slice(-1000);
            }

            // Auto hide log indicator after 500ms
            setTimeout(() => {
                nodes = nodes.map((n) => {
                    if (n.id === nodeId) {
                        return { ...n, data: { ...n.data, hasLogs: false } };
                    }
                    return n;
                });
            }, 500);
        }
    }

    onMount(() => {
        const result = yamlToGraph(yamlContent, viewJson || {});
        nodes = result.nodes.map((n: DmFlowNode) => ({
             ...n, 
             data: { ...n.data, status: "unknown" }
        }));
        edges = result.edges.map((e: DmFlowEdge) => ({
            ...e,
            style: "stroke: #94a3b8; stroke-width: 1.5",
        }));
        
        connectWs();
    });

    onDestroy(() => {
        if (ws) {
            ws.close();
            ws = null;
        }
    });
</script>

<div class="runtime-graph-wrapper relative w-full h-full min-h-[500px] border rounded-lg bg-card overflow-hidden">
    <SvelteFlow
        {nodes}
        {edges}
        {nodeTypes}
        {colorMode}
        fitView
        minZoom={0.2}
        nodesDraggable={false}
        nodesConnectable={false}
        elementsSelectable={true}
        onnodeclick={(e) => {
            selectedNode = e.node as DmFlowNode;
        }}
    >
        <Controls showLock={false} />
        <Background variant={BackgroundVariant.Dots} />
        <MiniMap />
    </SvelteFlow>
    
    <!-- Status overlay text -->
    <div class="absolute top-4 left-4 z-10 flex flex-col gap-1 pointer-events-none">
        <div class="px-3 py-1.5 bg-background/80 backdrop-blur-md border rounded-md shadow-sm font-mono text-xs font-semibold flex items-center gap-2">
            Status: 
            <span class="capitalize" class:text-green-500={globalStatus.toLowerCase() === 'running'} class:text-red-500={globalStatus.toLowerCase() === 'failed'}>
                {globalStatus}
            </span>
        </div>
    </div>
    
    {#if selectedNode}
        <NodeInspector
            node={selectedNode}
            logs={logsMap[selectedNode.id] || []}
            ioEvents={ioMap[selectedNode.id] || []}
            metrics={metricsMap.get(selectedNode.id)}
            onClose={() => selectedNode = null}
        />
    {/if}
</div>
