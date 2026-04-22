export type TerminalThemePresetId =
    | "auto"
    | "midnight"
    | "paper"
    | "forest"
    | "amber";

export type TerminalThemeSpec = {
    background: string;
    foreground: string;
    cursor: string;
    selectionBackground: string;
};

export type TerminalThemeOverrides = Partial<TerminalThemeSpec>;

type ModeName = "light" | "dark";

export const TERMINAL_THEME_FIELDS: Array<{
    key: keyof TerminalThemeSpec;
    label: string;
}> = [
    { key: "background", label: "Background" },
    { key: "foreground", label: "Foreground" },
    { key: "cursor", label: "Cursor" },
    { key: "selectionBackground", label: "Selection" },
];

export const TERMINAL_THEME_PRESET_META: Array<{
    id: TerminalThemePresetId;
    label: string;
    description: string;
}> = [
    { id: "auto", label: "Auto", description: "Follow light and dark mode" },
    { id: "midnight", label: "Midnight", description: "Deep blue console" },
    { id: "paper", label: "Paper", description: "Soft light background" },
    { id: "forest", label: "Forest", description: "Muted green terminal" },
    { id: "amber", label: "Amber", description: "Warm retro glow" },
];

const PRESET_THEMES: Record<
    Exclude<TerminalThemePresetId, "auto">,
    TerminalThemeSpec
> = {
    midnight: {
        background: "#0b1020",
        foreground: "#d7def7",
        cursor: "#8fb3ff",
        selectionBackground: "#253156",
    },
    paper: {
        background: "#f7f4ea",
        foreground: "#2d241c",
        cursor: "#8d5b2a",
        selectionBackground: "#ead7ba",
    },
    forest: {
        background: "#0d1714",
        foreground: "#d6efe4",
        cursor: "#77d9aa",
        selectionBackground: "#173126",
    },
    amber: {
        background: "#1a1209",
        foreground: "#ffd89a",
        cursor: "#ffb347",
        selectionBackground: "#4a3415",
    },
};

const AUTO_THEME_BY_MODE: Record<ModeName, TerminalThemeSpec> = {
    light: PRESET_THEMES.paper,
    dark: PRESET_THEMES.midnight,
};

export function resolveTerminalTheme(
    preset: TerminalThemePresetId = "auto",
    mode: ModeName = "dark",
    overrides: TerminalThemeOverrides = {},
): TerminalThemeSpec {
    const base =
        preset === "auto" ? AUTO_THEME_BY_MODE[mode] : PRESET_THEMES[preset];

    return {
        ...base,
        ...overrides,
    };
}
