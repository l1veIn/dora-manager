export type NodeOrigin = "builtin" | "git" | "local";

function installedAtValue(node: any): number {
    const raw = Number(node?.installed_at ?? 0);
    return Number.isFinite(raw) ? raw : 0;
}

export function isInstalledNode(node: any): boolean {
    return Boolean(node?.executable && String(node.executable).trim() !== "");
}

export function nodeRuntimeLabel(node: any): string {
    return node?.runtime?.language || "unknown";
}

export function nodeCategory(node: any): string {
    return node?.display?.category || "Uncategorized";
}

export function nodePrimaryMaintainer(node: any): string | null {
    return node?.maintainers?.[0]?.name || null;
}

export function nodeAvatarSrc(node: any): string | null {
    const avatar = node?.display?.avatar;
    if (!avatar) return null;
    const encoded = String(avatar)
        .split("/")
        .map((segment) => encodeURIComponent(segment))
        .join("/");
    return `/api/nodes/${encodeURIComponent(node.id)}/artifacts/${encoded}`;
}

export function nodeOrigin(node: any): NodeOrigin {
    const tags = node?.display?.tags || [];
    const category = node?.display?.category || "";
    const path = String(node?.path || "");

    if (
        tags.includes("builtin") ||
        category.startsWith("Builtin/") ||
        (path.includes("/nodes/") && !path.includes("/.dm/nodes/"))
    ) {
        return "builtin";
    }

    if (node?.source?.github || node?.repository?.url) {
        return "git";
    }

    return "local";
}

export function nodeOriginLabel(node: any): string {
    switch (nodeOrigin(node)) {
        case "builtin":
            return "Builtin";
        case "git":
            return "Git Import";
        case "local":
            return "Local";
    }
}

export function sortNodesForCatalog(nodes: any[]): any[] {
    return [...nodes].sort((a, b) => {
        const installedDelta =
            Number(isInstalledNode(b)) - Number(isInstalledNode(a));
        if (installedDelta !== 0) return installedDelta;

        const originWeight = (node: any) => {
            switch (nodeOrigin(node)) {
                case "local":
                    return 3;
                case "git":
                    return 2;
                case "builtin":
                    return 1;
            }
        };

        const originDelta = originWeight(b) - originWeight(a);
        if (originDelta !== 0) return originDelta;

        const installedAtDelta = installedAtValue(b) - installedAtValue(a);
        if (installedAtDelta !== 0) return installedAtDelta;

        return String(a?.id || "").localeCompare(String(b?.id || ""));
    });
}
