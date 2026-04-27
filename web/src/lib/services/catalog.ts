export type ServiceOrigin = "builtin" | "git" | "local";

function installedAtValue(service: any): number {
    const raw = Number(service?.installed_at ?? 0);
    return Number.isFinite(raw) ? raw : 0;
}

export function isInstalledService(service: any): boolean {
    if (service?.builtin) return true;
    return Boolean(service?.runtime?.exec && String(service.runtime.exec).trim() !== "");
}

export function serviceRuntimeLabel(service: any): string {
    return service?.runtime?.kind || "unknown";
}

export function serviceCategory(service: any): string {
    return service?.display?.category || "Uncategorized";
}

export function servicePrimaryMaintainer(service: any): string | null {
    return service?.maintainers?.[0]?.name || null;
}

export function serviceAvatarSrc(service: any): string | null {
    const avatar = service?.display?.avatar;
    if (!avatar) return null;
    const encoded = String(avatar)
        .split("/")
        .map((segment) => encodeURIComponent(segment))
        .join("/");
    return `/api/services/${encodeURIComponent(service.id)}/artifacts/${encoded}`;
}

export function serviceOrigin(service: any): ServiceOrigin {
    const tags = service?.display?.tags || [];
    const category = service?.display?.category || "";
    const path = String(service?.path || "");

    if (
        service?.builtin ||
        tags.includes("builtin") ||
        category.startsWith("Builtin/") ||
        (path.includes("/services/") && !path.includes("/.dm/services/"))
    ) {
        return "builtin";
    }

    if (service?.repository?.url) {
        return "git";
    }

    return "local";
}

export function serviceOriginLabel(service: any): string {
    switch (serviceOrigin(service)) {
        case "builtin":
            return "Builtin";
        case "git":
            return "Git Import";
        case "local":
            return "Local";
    }
}

export function sortServicesForCatalog(services: any[]): any[] {
    return [...services].sort((a, b) => {
        const installedDelta =
            Number(isInstalledService(b)) - Number(isInstalledService(a));
        if (installedDelta !== 0) return installedDelta;

        const originWeight = (service: any) => {
            switch (serviceOrigin(service)) {
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
