function titleCaseError(text: string): string {
    if (!text) return "";
    return text.charAt(0).toUpperCase() + text.slice(1);
}

export function summarizeOutcomeSummary(raw: string | null | undefined): string {
    const trimmed = raw?.trim();
    if (!trimmed) return "";
    if (!trimmed.includes("\n")) return trimmed;

    const normalized = trimmed.replace(/^Failed:\s*/, "");
    const lines = normalized
        .split("\n")
        .map((line) => line.trim())
        .filter(Boolean);

    const cleaned: string[] = [];
    let skippingLocation = false;

    for (const line of lines) {
        if (line.startsWith("dataflow start triggered:")) continue;
        if (line === "[ERROR]" || line === "Caused by:") continue;
        if (line === "Location:") {
            skippingLocation = true;
            continue;
        }
        if (skippingLocation) {
            skippingLocation = false;
            continue;
        }
        cleaned.push(line.replace(/^\d+:\s*/, ""));
    }

    const deduped = cleaned.filter(
        (line, index) => cleaned.indexOf(line) === index,
    );
    if (!deduped.length) return trimmed;

    const summary = titleCaseError(deduped[0]);
    const cause = deduped.find(
        (line, index) =>
            index > 0 &&
            !line.startsWith("failed to run `") &&
            !line.startsWith("Failed to run `"),
    );

    return cause ? `${summary} - ${cause}` : summary;
}
