<script lang="ts">
    import { onMount, tick } from 'svelte';
    import { GridStack } from 'gridstack';
    import 'gridstack/dist/gridstack.min.css';
    
    import type { WorkspaceGridItem } from "./types";
    import RootWidgetWrapper from "./widgets/RootWidgetWrapper.svelte";
    import { getPanelDefinition } from "./panels/registry";
    import type { PanelContext } from "./panels/types";

    let {
        layout = $bindable([]),
        runId,
        snapshots = [],
        inputValues = {},
        nodes = [],
        onEmit,
        onLayoutChange = () => {},
        refreshToken = 0,
        isRunActive = false,
    } = $props<{
        layout?: WorkspaceGridItem[];
        runId: string;
        snapshots?: any[];
        inputValues?: Record<string, any>;
        nodes?: any[];
        onEmit?: any;
        onLayoutChange?: (newLayout: WorkspaceGridItem[]) => void;
        refreshToken?: number;
        isRunActive?: boolean;
    }>();

    let gridServer: GridStack;
    let gridContainer: HTMLDivElement;
    let panelContext = $derived<PanelContext>({
        runId,
        snapshots,
        inputValues,
        nodes,
        refreshToken,
        isRunActive,
        emitMessage: onEmit ?? (async () => {}),
    });

    // We maintain an independent flat state proxy for the DOM items.
    let gridItems = $state<WorkspaceGridItem[]>(layout || []);

    onMount(() => {
        gridServer = GridStack.init({
            column: 12,
            cellHeight: 80,
            margin: 10,
            float: true, // Allow empty space between items vertically
            animate: true,
            handle: '.grid-drag-handle',
            resizable: { handles: 's, e, se' },
            alwaysShowResizeHandle: /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(
                navigator.userAgent
            )
        }, gridContainer);

        // Listen for internal drag/resize grid mutations
        gridServer.on('change', (event, items) => {
            if (!items) return;
            items.forEach((gsNode) => {
                const id = gsNode.id;
                // find DOM node mapping
                const matchIndex = gridItems.findIndex(i => i.id === id);
                if (matchIndex !== -1) {
                    gridItems[matchIndex].x = gsNode.x ?? 0;
                    gridItems[matchIndex].y = gsNode.y ?? 0;
                    gridItems[matchIndex].w = gsNode.w ?? 6;
                    gridItems[matchIndex].h = gsNode.h ?? 4;
                }
            });
            // Reflect mutations back up the tree
            layout = gridItems;
            onLayoutChange(gridItems);
        });

        // Whenever widgets are removed from DOM entirely via Svelte, GridStack catches it if auto-observed
    });

    // Reactive sync fallback
    $effect(() => {
        if (layout && layout !== gridItems) {
            gridItems = layout;
            // When widgets are dynamically updated via outside signals, Gridstack auto detects elements 
            // injected to the DOM and automatically mounts them via our gridWidget action.
        }
    });

    const api = {
        close: (nodeId: string) => {
            // Because our Svelte logic explicitly drives the DOM, simply filtering our state will trigger the destroy block.
            gridItems = gridItems.filter(i => i.id !== nodeId);
            layout = gridItems;
            onLayoutChange(gridItems);
        }
    };

    /**
     * Svelte Action to seamlessly weave Svelte '#each' components directly into GridStack without reactivity conflicts.
     * When Svelte creates the div, we tell GridStack to take physics control of it.
     */
    function gridWidget(node: HTMLElement, dataItem: WorkspaceGridItem) {
        node.setAttribute('gs-id', dataItem.id);
        node.setAttribute('gs-x', String(dataItem.x));
        node.setAttribute('gs-y', String(dataItem.y));
        node.setAttribute('gs-w', String(dataItem.w));
        node.setAttribute('gs-h', String(dataItem.h));

        tick().then(() => {
            if (gridServer) {
                gridServer.makeWidget(node);
            }
        });
        
        return {
            destroy() {
                if (gridServer) {
                    // false ensures GridStack removes its metadata but doesn't destruct the DOM (since Svelte destroys the DOM)
                    gridServer.removeWidget(node, false);
                }
            }
        };
    }

</script>

<div class="h-full w-full overflow-y-auto overflow-x-hidden bg-muted/10 relative pb-10">
    <div bind:this={gridContainer} class="grid-stack w-full h-full">
        <!-- Render existing items initially mapped from Grid stack schema attributes gs-x, gs-y... -->
        {#each gridItems as dataItem (dataItem.id)}
            {@const definition = getPanelDefinition(dataItem.widgetType)}
            {@const PanelComponent = definition.component}
            <div 
                class="grid-stack-item cursor-default" 
                use:gridWidget={dataItem}
            >
                <div class="grid-stack-item-content p-0.5 lg:p-1 overflow-hidden pointer-events-auto">
                    <!-- Standard inner wrapper provides visual chrome, buttons, title bar. -->
                    <RootWidgetWrapper item={dataItem} {definition} {api}>
                        <!-- Content boundary. Let it fill 100% and break appropriately. -->
                        <div class="w-full h-full relative overflow-hidden break-words">
                            <PanelComponent
                                item={dataItem}
                                {api}
                                context={panelContext}
                                onConfigChange={() => {
                                    layout = gridItems;
                                    onLayoutChange(gridItems);
                                }}
                            />
                        </div>
                    </RootWidgetWrapper>
                </div>
            </div>
        {/each}
    </div>
</div>

<style>
    /* Override gridstack default item-content padding if needed because we handle bounds in our root wrapper */
    :global(.grid-stack-item-content) {
        inset: 0 !important;
        position: absolute;
    }
    
    /* Make grid-stack parent expand indefinitely */
    :global(.grid-stack) {
        min-height: 100% !important;
    }
</style>
