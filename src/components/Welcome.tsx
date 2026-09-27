import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import type { ScanTick } from "../hooks/useScan";
import { api, type DiskInfo } from "../lib/api";
import { formatBytes, formatCount } from "../lib/format";

interface Props {
  home: string;
  progress: ScanTick | null;
  error: string;
  onScan: (path: string) => void;
  onCancel: () => void;
}

export function Welcome({ home, progress, error, onScan, onCancel }: Props) {
  const [disks, setDisks] = useState<DiskInfo[]>([]);

  useEffect(() => {
    api.listDisks().then(setDisks, () => setDisks([]));
  }, []);

  if (progress) {
    return (
      <div className="flex flex-1 flex-col items-center justify-center gap-6" role="status" aria-live="polite">
        <div className="relative h-40 w-40">
          <div className="absolute inset-0 animate-spin rounded-full border-4 border-transparent border-t-[var(--color-accent)] border-r-[var(--color-accent)]/40" />
          <div className="absolute inset-5 animate-[spin_2.4s_linear_infinite_reverse] rounded-full border-4 border-transparent border-b-[#8b5cd6]" />
          <div className="absolute inset-0 flex flex-col items-center justify-center">
            <span className="text-2xl font-semibold tabular-nums">{formatBytes(progress.bytes)}</span>
            <span className="text-xs text-[var(--color-muted)]">{formatCount(progress.files)} arquivos</span>
          </div>
        </div>
        <p className="text-[var(--color-muted)]">Mapeando o disco…</p>
        <button
          type="button"
          onClick={onCancel}
          className="rounded-md border border-[var(--color-line)] px-4 py-1.5 text-sm hover:bg-[var(--color-panel-2)]"
        >
          Cancelar
        </button>
      </div>
    );
  }

  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-8 p-8 text-center">
      <div>
        <div className="text-6xl">🧹</div>
        <h1 className="mt-4 text-4xl font-semibold tracking-tight">Onde está o seu espaço?</h1>
        <p className="mx-auto mt-3 max-w-lg text-[var(--color-muted)]">
          O Faxina mapeia uma pasta inteira, mostra o que é cache, build e duplicata — e some com o que você escolher.
        </p>
      </div>
      <div className="flex gap-3">
        <button
          type="button"
          disabled={!home}
          onClick={() => onScan(home)}
          className="rounded-xl bg-[var(--color-accent)] px-6 py-3 text-lg font-semibold text-black shadow-[0_0_40px_rgba(227,179,65,0.25)] hover:brightness-110"
        >
          Analisar minha pasta pessoal
        </button>
        <button
          type="button"
          onClick={async () => {
            const dir = await open({ directory: true });
            if (typeof dir === "string") onScan(dir);
          }}
          className="rounded-xl border border-[var(--color-line)] px-6 py-3 text-lg hover:bg-[var(--color-panel-2)]"
        >
          Escolher pasta…
        </button>
      </div>
      {disks.length > 0 && (
        <section aria-label="Discos" className="w-full max-w-3xl">
          <h2 className="mb-3 text-[11px] tracking-wider text-[var(--color-muted)] uppercase">
            Ou analise um disco inteiro
          </h2>
          <ul className="grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-3">
            {disks.map((d) => {
              const used = d.total - d.free;
              return (
                <li key={d.mount}>
                  <button
                    type="button"
                    onClick={() => onScan(d.mount)}
                    className="w-full rounded-xl border border-[var(--color-line)] bg-[var(--color-panel)] p-4 text-left hover:border-[var(--color-accent)]"
                  >
                    <div className="flex items-baseline justify-between">
                      <span className="text-lg font-semibold">💽 {d.mount}</span>
                      <span className="text-xs text-[var(--color-muted)]">{formatBytes(d.total)}</span>
                    </div>
                    <div className="mt-3 h-1.5 rounded bg-[var(--color-panel-2)]">
                      <div
                        className="h-full rounded bg-[var(--color-accent)]"
                        style={{ width: `${(used / d.total) * 100}%` }}
                      />
                    </div>
                    <div className="mt-2 text-xs text-[var(--color-muted)]">{formatBytes(d.free)} livres</div>
                  </button>
                </li>
              );
            })}
          </ul>
          <p className="mt-2 text-xs text-[var(--color-muted)]">
            Discos grandes levam alguns minutos; dá para cancelar a qualquer momento.
          </p>
        </section>
      )}
      {error && <p className="text-sm text-[var(--color-danger)]">{error}</p>}
    </div>
  );
}
