import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";

export type ManagedTerminal = {
    term: Terminal;
    fit: () => void;
    write: (text: string) => void;
    resetWithText: (text: string) => void;
    dispose: () => void;
};

function normalizeTerminalText(text: string): string {
    return text.replace(/\r?\n/g, "\r\n");
}

export function createManagedTerminal(container: HTMLElement): ManagedTerminal {
    const term = new Terminal({
        convertEol: true,
        disableStdin: true,
        fontFamily: '"JetBrains Mono", "Fira Code", ui-monospace, SFMono-Regular, Menlo, monospace',
        fontSize: 12,
        lineHeight: 1.35,
        scrollback: 5000,
        theme: {
            background: "#0b1020",
            foreground: "#d7def7",
            cursor: "#8fb3ff",
            selectionBackground: "#253156",
        },
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
        dispose: () => term.dispose(),
    };
}
