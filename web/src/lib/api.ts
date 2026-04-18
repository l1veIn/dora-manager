export const API_BASE = '/api';

export class ApiError extends Error {
    status: number;
    rawMessage: string;
    details?: unknown;

    constructor({
        status,
        message,
        rawMessage,
        details,
    }: {
        status: number;
        message: string;
        rawMessage: string;
        details?: unknown;
    }) {
        super(message);
        this.name = 'ApiError';
        this.status = status;
        this.rawMessage = rawMessage;
        this.details = details;
    }
}

function normalizeErrorText(text: string): string {
    return text.trim().replace(/^Error:\s*/i, '').replace(/^"+|"+$/g, '');
}

function extractErrorMessage(payload: unknown): string | null {
    if (typeof payload === 'string') {
        return normalizeErrorText(payload);
    }

    if (!payload || typeof payload !== 'object') {
        return null;
    }

    const record = payload as Record<string, unknown>;
    for (const key of ['error', 'message', 'detail']) {
        const value = record[key];
        if (typeof value === 'string' && value.trim()) {
            return normalizeErrorText(value);
        }
    }

    return null;
}

async function readError(res: Response): Promise<never> {
    const text = (await res.text()).trim();
    const fallbackMessage = `${res.status} ${res.statusText}`.trim();
    if (!text) {
        throw new ApiError({
            status: res.status,
            message: fallbackMessage,
            rawMessage: fallbackMessage,
        });
    }

    let details: unknown;
    let message = normalizeErrorText(text);

    try {
        details = JSON.parse(text);
        message = extractErrorMessage(details) ?? message;
    } catch {
        // Fall through to the raw text body.
    }

    throw new ApiError({
        status: res.status,
        message: message || fallbackMessage,
        rawMessage: text,
        details,
    });
}

export async function get<T>(path: string): Promise<T> {
    const res = await fetch(`${API_BASE}${path}`);
    if (!res.ok) return readError(res);
    return res.json();
}

export async function getText(path: string): Promise<string> {
    const res = await fetch(`${API_BASE}${path}`);
    if (!res.ok) return readError(res);
    return res.text();
}

export async function post<T>(path: string, body?: unknown): Promise<T> {
    const res = await fetch(`${API_BASE}${path}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: body ? JSON.stringify(body) : undefined,
    });
    if (!res.ok) return readError(res);
    return res.json();
}

export async function del<T>(path: string): Promise<T> {
    const res = await fetch(`${API_BASE}${path}`, {
        method: 'DELETE',
    });
    if (!res.ok) return readError(res);
    return res.json();
}
