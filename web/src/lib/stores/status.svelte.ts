import { get } from '$lib/api';

// Global reactive state for dora runtime status
let status = $state<any>(null);
let doctor = $state<any>(null);
let nodes = $state<any[]>([]);
let loading = $state(true);

async function refresh(showSkeleton = false) {
    if (showSkeleton) {
        loading = true;
        status = null;
        doctor = null;
        nodes = [];
    }
    try {
        [status, doctor, nodes] = await Promise.all([
            get('/status').catch(() => null),
            get('/doctor').catch(() => null),
            get('/nodes').catch(() => []),
        ] as any[]);
    } finally {
        loading = false;
    }
}

export function useStatus() {
    return {
        get status() { return status; },
        get doctor() { return doctor; },
        get nodes() { return nodes; },
        get loading() { return loading; },
        refresh,
    };
}
