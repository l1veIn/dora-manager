import { get } from "$lib/api";

export type MessageFilterState = {
    nodes: string[];
    tags: string[];
};

export type SnapshotFilterState = {
    nodes: string[];
    tags: string[];
};

export function summarizeSelection(values: string[], allLabel: string) {
    if (values.includes("*")) return allLabel;
    if (values.length === 1) return values[0];
    if (values.length === 2) return values.join(", ");
    return `${values.length} selected`;
}

export function createMessageHistoryState(runId: () => string, filters: () => MessageFilterState) {
    let messages = $state<any[]>([]);
    let fetching = $state(false);
    let fetchingOld = $state(false);
    let hasMoreOld = $state(true);

    let oldestSeq = $derived(messages.length > 0 ? messages[0].seq : null);
    let newestSeq = $derived(messages.length > 0 ? messages[messages.length - 1].seq : null);

    function buildUrl(params: Record<string, string | number | boolean>) {
        const url = new URL(`/runs/${runId()}/messages`, window.location.origin);
        for (const [key, value] of Object.entries(params)) {
            url.searchParams.set(key, String(value));
        }
        const currentFilters = filters();
        if (!currentFilters.nodes.includes("*")) {
            url.searchParams.set("from", currentFilters.nodes.join(","));
        }
        if (!currentFilters.tags.includes("*")) {
            url.searchParams.set("tag", currentFilters.tags.join(","));
        }
        return `${url.pathname}${url.search}`;
    }

    async function loadInitial(onLoaded?: () => void) {
        if (!runId() || fetching) return;
        fetching = true;
        try {
            const res: any = await get(buildUrl({ limit: 50, desc: true }));
            if (res?.messages) {
                messages = res.messages;
                hasMoreOld = res.messages.length === 50;
                onLoaded?.();
            }
        } finally {
            fetching = false;
        }
    }

    async function loadNew(onLoaded?: () => void) {
        if (!runId() || fetching || newestSeq === null) return;
        fetching = true;
        try {
            const res: any = await get(buildUrl({ after_seq: newestSeq, limit: 50 }));
            if (res?.messages?.length) {
                messages = [...messages, ...res.messages];
                onLoaded?.();
            }
        } finally {
            fetching = false;
        }
    }

    async function loadOld(onLoaded?: (previousHeight: number) => void, previousHeight = 0) {
        if (!runId() || fetchingOld || oldestSeq === null || !hasMoreOld) return;
        fetchingOld = true;
        try {
            const res: any = await get(buildUrl({ before_seq: oldestSeq, limit: 50, desc: true }));
            if (res?.messages?.length) {
                hasMoreOld = res.messages.length === 50;
                messages = [...res.messages, ...messages];
                onLoaded?.(previousHeight);
            } else {
                hasMoreOld = false;
            }
        } finally {
            fetchingOld = false;
        }
    }

    function reset() {
        messages = [];
        hasMoreOld = true;
    }

    return {
        get messages() {
            return messages;
        },
        get fetching() {
            return fetching;
        },
        get fetchingOld() {
            return fetchingOld;
        },
        get hasMoreOld() {
            return hasMoreOld;
        },
        get oldestSeq() {
            return oldestSeq;
        },
        get newestSeq() {
            return newestSeq;
        },
        loadInitial,
        loadNew,
        loadOld,
        reset,
    };
}

export function createSnapshotViewState(
    snapshots: () => any[],
    filters: () => SnapshotFilterState,
) {
    let filtered = $derived(
        snapshots()
            .filter((snapshot: any) => {
                const currentFilters = filters();
                const matchesNode =
                    currentFilters.nodes.includes("*") ||
                    currentFilters.nodes.includes(snapshot.node_id);
                const matchesTag =
                    currentFilters.tags.includes("*") ||
                    currentFilters.tags.includes(snapshot.tag);
                return matchesNode && matchesTag;
            }),
    );

    return {
        get snapshots() {
            return filtered;
        },
    };
}
