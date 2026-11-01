import { open } from "@tauri-apps/plugin-dialog";
import { type KeyboardEvent as ReactKeyboardEvent, useEffect, useRef, useState } from "react";
import type { useScan } from "../hooks/useScan";
import { api, type DiskInfo } from "../lib/api";
import { formatBytes } from "../lib/format";
import type { Mark } from "../lib/marks";
import { Collector } from "./Collector";
import Icon from "./Icon";
import { MapView } from "./MapView";
import ScanProgress from "./ScanProgress";

export default function DiskMap({
  active,
  scan,
  home,
  marked,
  onToggleMark,
  onClearMarked,
  onUnmark,
  onDeleteMarked,
}: {
  active: boolean;
  scan: ReturnType<typeof useScan>;
  home: string;
  marked: Map<string, Mark>;
  onToggleMark: (m: Mark) => void;
  onClearMarked: () => void;
  onUnmark: (path: string) => void;
  onDeleteMarked: (permanent: boolean) => void;
}) {
  const [disks, setDisks] = useState<DiskInfo[]>([]);
  const [expanded, setExpanded] = useState(false);
  const screen = useRef<HTMLElement>(null);
  const [notice, setNotice] = useState("");

  useEffect(() => {
    api.listDisks().then(setDisks, () => setDisks([]));
  }, []);

  // Opening the full-screen map moves focus into it, so keyboard users are not left behind.
  useEffect(() => {
    if (expanded) screen.current?.querySelector<HTMLElement>("button")?.focus();
  }, [expanded]);

  useEffect(() => {
    if (!active || !expanded) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !document.querySelector("dialog[open]")) setExpanded(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [active, expanded]);

  if (!active) return null;

  // While the map is full screen it behaves as a modal: Tab cycles inside it and never reaches the page behind.
  const trapFocus = (e: ReactKeyboardEvent<HTMLElement>) => {
    if (e.key !== "Tab") return;
    const items = Array.from(
      e.currentTarget.querySelectorAll<HTMLElement>('button, [href], input, select, textarea, [tabindex="0"]'),
    ).filter((el) => !el.hasAttribute("disabled"));
    if (!items.length) return;
    const first = items[0];
    const last = items[items.length - 1];
    const current = document.activeElement;
    const outside = !e.currentTarget.contains(current);
    if (e.shiftKey && (current === first || outside)) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && (current === last || outside)) {
      e.preventDefault();
      first.focus();
    }
  };
  // A failed scan or folder open is stored in the hook; show it where the picker's errors go.
  const message = notice || scan.error;

  const chooseFolder = async () => {
    try {
      const dir = await open({ directory: true });
      if (typeof dir === "string") {
        onClearMarked();
        setNotice("");
        scan.scan(dir);
      }
    } catch (e) {
      setNotice(`Não foi possível abrir o seletor: ${e}`);
    }
  };

  const markedList = [...marked.values()];

  return (
    // aria-modal is valid only with role="dialog", which is set on the same element in the same render.
    // biome-ignore lint/a11y/useAriaPropsSupportedByRole: the role is conditional, so the rule cannot see it
    <section
      ref={screen}
      className={`disk-map-screen ${expanded ? "is-expanded" : ""} flex flex-col gap-6 p-6 sm:p-8`}
      aria-label="Explorador visual do disco"
      role={expanded ? "dialog" : undefined}
      aria-modal={expanded || undefined}
      onKeyDown={expanded ? trapFocus : undefined}
    >
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <p className="text-[11px] font-bold tracking-widest text-amber-400 uppercase">O mapa que você já conhece</p>
          <h1 className="mt-1 text-3xl font-extrabold tracking-tight text-white sm:text-4xl">
            O disco inteiro, à vista.
          </h1>
          <p className="mt-1 text-sm text-slate-400">
            Blocos, anéis e cada pasta no seu lugar. Explore antes de decidir.
          </p>
        </div>
        <button
          type="button"
          className="inline-flex cursor-pointer items-center gap-2 rounded-xl border border-slate-700 bg-slate-800/80 px-4 py-2.5 text-xs font-semibold text-slate-200 transition-colors hover:bg-slate-750"
          onClick={() => setExpanded((value) => !value)}
        >
          <Icon name="grid" size={16} />
          <span>{expanded ? "Sair da tela cheia" : "Expandir mapa"}</span>
        </button>
      </div>

      <div className="flex flex-wrap items-center justify-between gap-4 rounded-2xl border border-slate-800/80 bg-slate-900/60 p-4 shadow-md backdrop-blur-md">
        {disks.length > 0 && (
          <label className="flex items-center gap-3 text-xs font-semibold text-slate-300">
            <span>Disco</span>
            <select
              aria-label="Escolher disco do mapa"
              value={scan.root}
              disabled={scan.progress !== null}
              onChange={(e) => {
                onClearMarked();
                setNotice("");
                scan.scan(e.target.value);
              }}
              className="cursor-pointer rounded-xl border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-slate-200 outline-none focus:border-amber-500 disabled:opacity-50"
            >
              {!disks.some((d) => d.mount === scan.root) && scan.root && <option value={scan.root}>{scan.root}</option>}
              {disks.map((disk) => (
                <option key={disk.mount} value={disk.mount}>
                  Unidade {disk.mount} ({formatBytes(disk.free)} livres de {formatBytes(disk.total)})
                </option>
              ))}
            </select>
          </label>
        )}
        <div className="flex flex-wrap items-center gap-2.5">
          <button
            type="button"
            className="cursor-pointer rounded-xl border border-slate-700 bg-slate-800/80 px-4 py-2 text-xs font-semibold text-slate-200 transition-colors hover:bg-slate-750 disabled:opacity-50"
            disabled={scan.progress !== null}
            onClick={chooseFolder}
          >
            Escolher outra pasta…
          </button>
          {scan.view && (
            <button
              type="button"
              className="inline-flex cursor-pointer items-center gap-1.5 rounded-xl border border-slate-700 bg-slate-800/80 px-4 py-2 text-xs font-semibold text-slate-200 transition-colors hover:bg-slate-750 disabled:opacity-50"
              disabled={scan.progress !== null}
              onClick={() => scan.scan(scan.root)}
            >
              <Icon name="refresh" size={14} />
              <span>Reanalisar</span>
            </button>
          )}
        </div>
      </div>

      {message && (
        <div
          className="rounded-2xl border border-rose-500/30 bg-rose-500/10 p-4 text-xs font-medium text-rose-200"
          role="alert"
        >
          {message}
        </div>
      )}

      {scan.progress ? (
        <ScanProgress progress={scan.progress} path={scan.scanning} onCancel={api.cancelScan} />
      ) : scan.view ? (
        <div className="native-map flex min-h-[520px] flex-1 flex-col overflow-hidden rounded-2xl border border-slate-800/80 bg-slate-900/60 shadow-xl backdrop-blur-md">
          <MapView scan={scan} marked={marked} onToggleMark={onToggleMark} />
          <Collector items={markedList} onClear={onClearMarked} onUnmark={onUnmark} onDelete={onDeleteMarked} />
        </div>
      ) : (
        <div className="flex flex-col items-center justify-center gap-6 rounded-3xl border border-slate-800/80 bg-slate-900/60 p-12 text-center shadow-lg backdrop-blur-md">
          <span className="flex h-20 w-20 items-center justify-center rounded-3xl border border-slate-700 bg-slate-800/80 text-4xl shadow-md">
            💽
          </span>
          <div className="max-w-md">
            <h2 className="text-xl font-bold text-white">Selecione uma unidade ou pasta para mapear</h2>
            <p className="mt-1 text-sm text-slate-400">
              O mapa visual analisa o tamanho real de cada pasta e arquivo em blocos e anéis interativos.
            </p>
          </div>
          <div className="flex flex-wrap items-center justify-center gap-3">
            {home && (
              <button
                type="button"
                className="cursor-pointer rounded-xl bg-gradient-to-r from-amber-500 to-amber-400 px-5 py-2.5 text-xs font-bold text-slate-950 shadow-lg shadow-amber-500/20 hover:brightness-110"
                onClick={() => scan.scan(home)}
              >
                Mapear pasta pessoal
              </button>
            )}
            {disks.map((d) => (
              <button
                key={d.mount}
                type="button"
                className="cursor-pointer rounded-xl border border-slate-700 bg-slate-800/80 px-4 py-2.5 text-xs font-semibold text-slate-200 hover:bg-slate-750"
                onClick={() => scan.scan(d.mount)}
              >
                Mapear disco {d.mount}
              </button>
            ))}
            <button
              type="button"
              className="cursor-pointer rounded-xl border border-slate-700 bg-slate-800/80 px-4 py-2.5 text-xs font-semibold text-slate-200 hover:bg-slate-750"
              onClick={chooseFolder}
            >
              Escolher pasta…
            </button>
          </div>
        </div>
      )}
    </section>
  );
}
