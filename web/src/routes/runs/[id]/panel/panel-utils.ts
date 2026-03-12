export type NormalizedOption = { value: string; label: string };

export type WidgetOverrides = {
    options?: NormalizedOption[];
    disabled?: boolean;
    label?: string;
    value?: any;
    variant?: string;
    loading?: boolean;
    progress?: number; // 0-1
};

export function normalizeOptions(
    items: any[],
    valueKey = "value",
    labelKey = "label",
): NormalizedOption[] {
    return items.map((item) =>
        typeof item === "string"
            ? { value: item, label: item }
            : {
                  value: String(
                      item[valueKey] ?? item.id ?? item.value ?? "",
                  ),
                  label: String(
                      item[labelKey] ??
                          item.label ??
                          item.name ??
                          item[valueKey] ??
                          item.id ??
                          "",
                  ),
              },
    );
}

export function smartResolve(
    xw: any,
    assets: Record<string, any>,
): WidgetOverrides {
    const bindId = xw?.bind;
    if (!bindId || typeof bindId !== "string") return {};

    const asset = assets[bindId];
    if (!asset?.data) return {};

    try {
        const data = JSON.parse(asset.data);

        if (Array.isArray(data))
            return {
                options: normalizeOptions(data, xw.valueKey, xw.labelKey),
            };

        if (typeof data === "boolean") return { disabled: !data };
        if (typeof data === "string") return { label: data };
        if (typeof data === "number") return { value: data };

        if (typeof data === "object" && data !== null) {
            const result: WidgetOverrides = { ...data };
            if (Array.isArray(data.options))
                result.options = normalizeOptions(
                    data.options,
                    xw.valueKey,
                    xw.labelKey,
                );
            return result;
        }
    } catch {}
    return {};
}

export function resolveDefault(
    def: any,
    opts: NormalizedOption[],
    assets: Record<string, any>,
): string | undefined {
    const xw = def?.["x-widget"];
    const bindId = xw?.bind;
    if (typeof bindId === "string") {
        const asset = assets[bindId];
        if (asset?.data) {
            try {
                let parsed = JSON.parse(asset.data);
                if (
                    typeof parsed === "object" &&
                    !Array.isArray(parsed) &&
                    Array.isArray(parsed.options)
                ) {
                    parsed = parsed.options;
                }
                if (Array.isArray(parsed)) {
                    const marked = parsed.find(
                        (item: any) => item.default === true,
                    );
                    if (marked) {
                        const vk = xw.valueKey ?? "value";
                        return String(
                            marked[vk] ?? marked.id ?? marked.value ?? "",
                        );
                    }
                }
            } catch {}
        }
    }
    if (def?.default != null) {
        const yamlDefault = String(def.default);
        if (opts.some((o) => o.value === yamlDefault)) return yamlDefault;
    }
    return opts[0]?.value;
}

/**
 * Resolve options for select/radio/checkbox widgets.
 * Uses overrides from smartResolve first, falls back to static xw.options.
 */
export function resolveOptions(
    xw: any,
    overrides: WidgetOverrides,
): NormalizedOption[] {
    return (
        overrides.options ??
        (Array.isArray(xw.options)
            ? normalizeOptions(xw.options, xw.valueKey, xw.labelKey)
            : [])
    );
}

export function parseHotkey(hotkey: string) {
    const parts = hotkey.toLowerCase().split("+").map((p) => p.trim());
    return {
        ctrl: parts.includes("ctrl") || parts.includes("control"),
        meta: parts.includes("meta") || parts.includes("cmd"),
        alt: parts.includes("alt"),
        shift: parts.includes("shift"),
        key: parts.filter(
            (p) =>
                !["ctrl", "control", "meta", "cmd", "alt", "shift"].includes(
                    p,
                ),
        )[0] || "",
    };
}

export function matchesHotkey(
    e: KeyboardEvent,
    parsed: ReturnType<typeof parseHotkey>,
): boolean {
    const ctrlOrMeta = parsed.ctrl || parsed.meta;
    if (ctrlOrMeta && !(e.ctrlKey || e.metaKey)) return false;
    if (!ctrlOrMeta && (e.ctrlKey || e.metaKey)) return false;
    if (parsed.alt !== e.altKey) return false;
    if (parsed.shift !== e.shiftKey) return false;
    return e.key.toLowerCase() === parsed.key;
}

export function formatHotkey(hotkey: string): string {
    const isMac =
        typeof navigator !== "undefined" && /Mac/i.test(navigator.userAgent);
    return hotkey
        .split("+")
        .map((p) => {
            const k = p.trim().toLowerCase();
            if (k === "ctrl" || k === "control") return isMac ? "⌃" : "Ctrl";
            if (k === "meta" || k === "cmd") return isMac ? "⌘" : "Ctrl";
            if (k === "alt") return isMac ? "⌥" : "Alt";
            if (k === "shift") return isMac ? "⇧" : "Shift";
            if (k === "enter") return "↵";
            if (k === "space") return "␣";
            if (k === "escape" || k === "esc") return "Esc";
            return p.trim().toUpperCase();
        })
        .join(isMac ? "" : "+");
}
