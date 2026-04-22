import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import type { TerminalThemeSpec } from "./themes";

export type ManagedTerminal = {
    term: Terminal;
    fit: () => void;
    write: (text: string) => void;
    resetWithText: (text: string) => void;
    setTheme: (theme: TerminalThemeSpec) => void;
    dispose: () => void;
};

function normalizeTerminalText(text: string): string {
    return text.replace(/\r?\n/g, "\r\n");
}

export function createManagedTerminal(
    container: HTMLElement,
    theme: TerminalThemeSpec,
): ManagedTerminal {
    const term = new Terminal({
        convertEol: true,
        disableStdin: true,
        fontFamily: '"JetBrains Mono", "Fira Code", ui-monospace, SFMono-Regular, Menlo, monospace',
        fontSize: 12,
        lineHeight: 1.35,
        scrollback: 5000,
        theme,
    });
    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(container);
    fitAddon.fit();

    return {
        term,
        fit: () => fitAddon.fit(),
        write: (text: string) => {
            if (!text) return;
            term.write(normalizeTerminalText(text));
        },
        resetWithText: (text: string) => {
            term.reset();
            if (!text) return;
            term.write(normalizeTerminalText(text));
        },
        setTheme: (nextTheme: TerminalThemeSpec) => {
            term.options.theme = nextTheme;
        },
        dispose: () => term.dispose(),
    };
}
