import { useCallback, useEffect, useMemo, useState } from "react";
import { BlackHole } from "./components/BlackHole";
import { Collector } from "./components/Collector";
import { DupesPanel } from "./components/DupesPanel";
import { JunkPanel } from "./components/JunkPanel";
import { MapView } from "./components/MapView";
import { UpdateBanner } from "./components/UpdateBanner";
import { Welcome } from "./components/Welcome";
import { useHole } from "./hooks/useHole";
import { useScan } from "./hooks/useScan";
import { api } from "./lib/api";
import type { Mark } from "./lib/marks";
import { getMotionPref, MOTION_LABELS, type MotionPref, setMotionPref } from "./lib/motion";

type Tab = "map" | "junk" | "dupes";

const TABS: { id: Tab; label: string }[] = [
  { id: "map", label: "Mapa do disco" },
  { id: "junk", label: "Limpeza rápida" },
  { id: "dupes", label: "Duplicatas" },
];

export default function App() {
  const scan = useScan();
  const { hole, swallow, finish } = useHole();
  const [tab, setTab] = useState<Tab>("map");
  const [home, setHome] = useState("");
  const [marked, setMarked] = useState<Map<string, Mark>>(new Map());
  const [motion, setMotion] = useState<MotionPref>(getMotionPref);

  const cycleMotion = () => {
    const next: MotionPref = motion === "system" ? "always" : motion === "always" ? "never" : "system";
    setMotionPref(next);
    setMotion(next);
  };

  useEffect(() => {
    api.homeDir().then(setHome);
  }, []);

  const toggleMark = useCallback((m: Mark) => {
    setMarked((prev) => {
      const next = new Map(prev);
      if (next.has(m.path)) next.delete(m.path);
      else {
        // Marking a folder supersedes anything already marked inside it.
        for (const p of next.keys()) if (p.startsWith(`${m.path}\\`) || p.startsWith(`${m.path}/`)) next.delete(p);
        next.set(m.path, { path: m.path, name: m.name, size: m.size, kind: m.kind });
      }
      return next;
    });
  }, []);

  const unmark = useCallback((path: string) => {
    setMarked((prev) => {
      const next = new Map(prev);
      next.delete(path);
      return next;
    });
  }, []);

  const deleteMarked = (permanent: boolean) => {
    const paths = [...marked.keys()];
    swallow(
      paths,
      async () => {
        const r = await api.deletePaths(paths, permanent);
        setMarked((prev) => new Map([...prev].filter(([p]) => !r.deleted.includes(p))));
        return { freed: r.freed, failed: r.failed.length, reason: r.failed[0]?.error };
      },
      scan.refresh,
    );
  };

  const items = useMemo(() => [...marked.values()], [marked]);

  return (
    <div className="flex h-full flex-col">
      <UpdateBanner />
      <header className="flex items-center gap-6 border-b border-[var(--color-line)] px-5 py-3">
        <div className="flex items-center gap-2 text-xl font-semibold">
          <span aria-hidden className="grid grid-cols-2 gap-0.5">
            <span className="h-2 w-2 rounded-[2px] bg-[#4f7dd6]" />
            <span className="h-2 w-2 rounded-[2px] bg-[#c9a13b]" />
            <span className="h-2 w-2 rounded-[2px] bg-[#3f9f68]" />
            <span className="h-2 w-2 rounded-[2px] bg-[#c4506a]" />
          </span>
          faxina
        </div>
        <nav className="flex gap-1" aria-label="Seções">
          {TABS.map((t) => (
            <button
              key={t.id}
              type="button"
              aria-current={tab === t.id ? "page" : undefined}
              onClick={() => setTab(t.id)}
              className={`rounded-md px-3 py-1.5 text-sm ${tab === t.id ? "bg-[var(--color-panel-2)] text-white" : "text-[var(--color-muted)] hover:text-white"}`}
            >
              {t.label}
            </button>
          ))}
        </nav>
        <button
          type="button"
          onClick={cycleMotion}
          title="Animação de exclusão: seguir o sistema, sempre ou nunca"
          className="ml-auto rounded-md px-3 py-1.5 text-sm text-[var(--color-muted)] hover:text-white"
        >
          <span aria-hidden="true">✨ </span>Animações: {MOTION_LABELS[motion]}
        </button>
        {tab === "map" && scan.view && (
          <button
            type="button"
            onClick={() => {
              setMarked(new Map());
              scan.reset();
            }}
            className="rounded-md border border-[var(--color-line)] px-3 py-1.5 text-sm hover:bg-[var(--color-panel-2)]"
          >
            <span aria-hidden="true">💽 </span>Trocar disco/pasta
          </button>
        )}
        {tab === "map" && scan.view && (
          <button
            type="button"
            onClick={() => scan.scan(scan.root)}
            className="rounded-md border border-[var(--color-line)] px-3 py-1.5 text-sm hover:bg-[var(--color-panel-2)]"
          >
            ↻ Reanalisar
          </button>
        )}
      </header>

      <main className="flex min-h-0 flex-1 flex-col">
        {tab === "map" &&
          (scan.view && !scan.progress ? (
            <MapView scan={scan} marked={marked} onToggleMark={toggleMark} />
          ) : (
            <Welcome
              home={home}
              progress={scan.progress}
              target={scan.scanning}
              error={scan.error}
              onScan={scan.scan}
              onCancel={api.cancelScan}
            />
          ))}
        {tab === "junk" && <JunkPanel swallow={swallow} />}
        {tab === "dupes" && <DupesPanel swallow={swallow} defaultRoot={scan.root || home} />}
      </main>

      {tab === "map" && (
        <Collector items={items} onClear={() => setMarked(new Map())} onUnmark={unmark} onDelete={deleteMarked} />
      )}
      {hole && (
        <BlackHole
          sources={hole.sources}
          freed={hole.freed}
          failed={hole.failed}
          reason={hole.reason}
          onDone={finish}
        />
      )}
    </div>
  );
}
