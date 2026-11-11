import { check } from "@tauri-apps/plugin-updater";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { flushSync } from "react-dom";
import { BigFiles } from "./components/BigFiles";
import { BlackHole } from "./components/BlackHole";
import CommandPalette, { type PaletteCommand } from "./components/CommandPalette";
import ConfirmDialog from "./components/ConfirmDialog";
import Coverage from "./components/Coverage";
import DiskMap from "./components/DiskMap";
import { DupesPanel } from "./components/DupesPanel";
import { EmptyPanel } from "./components/EmptyPanel";
import Growth from "./components/Growth";
import Icon from "./components/Icon";
import Orbit from "./components/Orbit";
import Overview from "./components/Overview";
import Rail from "./components/Rail";
import ReportPanel from "./components/ReportPanel";
import Review from "./components/Review";
import Routine from "./components/Routine";
import SpaceTabs from "./components/SpaceTabs";
import Updates from "./components/Updates";
import { WorktreesPanel } from "./components/WorktreesPanel";
import {
  BASE_RULES,
  createReport,
  isCleanupDue,
  mergeRulesWithScan,
  type Page,
  type Rule,
  readHistory,
  readSettings,
  recommended,
  saveHistory,
  saveSettings,
  totalSize,
} from "./domain";
import { useHole } from "./hooks/useHole";
import { useScan } from "./hooks/useScan";
import { api, type DeleteReport, type DiskInfo } from "./lib/api";
import { flyToOrbit, trackPointer } from "./lib/fly";
import { formatBytes } from "./lib/format";
import { kindColor } from "./lib/kinds";
import type { Mark } from "./lib/marks";
import { getMotionPref, MOTION_LABELS, type MotionPref, setMotionPref, shouldAnimate } from "./lib/motion";
import { type OrbitItem, planOrbit } from "./lib/orbit";
import { platformName } from "./lib/platform";
import { PAGE_COMMANDS, SPACES, type Space, spaceOf } from "./lib/spaces";

export default function App() {
  const [page, setPageState] = useState<Page>("overview");
  const [palette, setPalette] = useState(false);
  // Each space reopens on the tab the user left it at.
  const lastTab = useRef<Partial<Record<Space, Page>>>({});
  const setPage = useCallback((next: Page) => {
    lastTab.current[spaceOf(next).id] = next;
    const apply = () => flushSync(() => setPageState(next));
    // The stage morphs between pages where the webview supports view transitions; elsewhere it just swaps.
    const doc = document as Document & { startViewTransition?: (cb: () => void) => unknown };
    if (doc.startViewTransition && shouldAnimate()) doc.startViewTransition(apply);
    else apply();
  }, []);
  const goSpace = (s: Space) => {
    const def = SPACES.find((d) => d.id === s) ?? SPACES[0];
    setPage(lastTab.current[s] ?? def.tabs[0].page);
  };
  const [settings, setSettings] = useState(readSettings);
  const [history, setHistory] = useState(readHistory);
  const [rules, setRules] = useState<Rule[]>(BASE_RULES);
  const [scanned, setScanned] = useState(false);
  const [junkLoading, setJunkLoading] = useState(false);
  const [scanError, setScanError] = useState(false);
  const [storageError, setStorageError] = useState(false);
  const [notice, setNotice] = useState("");
  const [selected, setSelected] = useState<string[]>([]);
  const [detail, setDetail] = useState<Rule>(BASE_RULES[0]);
  const [confirm, setConfirm] = useState(false);

  const [home, setHome] = useState("");
  const [disks, setDisks] = useState<DiskInfo[]>([]);
  const [diskMount, setDiskMount] = useState("");
  const [marked, setMarked] = useState<Map<string, Mark>>(new Map());
  // What the Grandes, Duplicatas, Vazios and Repositórios panels put in the Órbita.
  const [queued, setQueued] = useState<Map<string, OrbitItem>>(new Map());
  const [epoch, setEpoch] = useState(0);
  const [updateReady, setUpdateReady] = useState(false);
  const [motion, setMotion] = useState<MotionPref>(getMotionPref);

  const scan = useScan();
  const { hole, swallow, finish } = useHole();
  const main = useRef<HTMLElement>(null);

  const cycleMotion = () => {
    const next: MotionPref = motion === "system" ? "always" : motion === "always" ? "never" : "system";
    setMotionPref(next);
    setMotion(next);
  };

  useEffect(() => trackPointer(), []);
  // A quiet look for a new version marks Ajustes; the Atualizações tab does the real install.
  useEffect(() => {
    const t = setTimeout(() => {
      check({ timeout: 15000 }).then(
        (u) => setUpdateReady(!!u),
        () => setUpdateReady(false),
      );
    }, 4000);
    return () => clearTimeout(t);
  }, []);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setPalette((v) => !v);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // Read through a ref so changing a setting does not re-run the whole junk scan.
  const settingsRef = useRef(settings);
  useEffect(() => {
    settingsRef.current = settings;
  }, [settings]);

  // Each analysis takes a ticket; cancelling takes a new one, so the abandoned result is dropped.
  const junkTicket = useRef(0);
  const junkScanId = useRef<number | null>(null);
  const loadJunk = useCallback(async () => {
    const mine = ++junkTicket.current;
    setJunkLoading(true);
    setScanError(false);
    try {
      const { items, scanId } = await api.junkScan();
      if (mine !== junkTicket.current) return;
      junkScanId.current = scanId;
      const merged = mergeRulesWithScan(items);
      setRules(merged);
      setScanned(true);
      const rec = recommended(merged, settingsRef.current).map((r) => r.id);
      setSelected(rec);
      const firstActive = merged.find((r) => r.size > 0 && !r.blocked) || merged[0];
      setDetail(firstActive);
      setNotice(
        rec.length ? "Análise concluída. Revise os itens para liberar espaço." : "Análise concluída: tudo limpo!",
      );
    } catch {
      if (mine === junkTicket.current) setScanError(true);
    } finally {
      if (mine === junkTicket.current) setJunkLoading(false);
    }
  }, []);

  useEffect(() => {
    api.homeDir().then(setHome, () => setHome(""));
    api.listDisks().then(setDisks, () => setDisks([]));
    loadJunk();
  }, [loadJunk]);

  useEffect(() => {
    const okSettings = saveSettings(settings);
    const okHistory = saveHistory(history);
    setStorageError(!okSettings || !okHistory);
  }, [settings, history]);

  useEffect(() => {
    if (page) main.current?.focus({ preventScroll: true });
  }, [page]);

  const markedRef = useRef(marked);
  markedRef.current = marked;
  const toggleMark = useCallback((m: Mark) => {
    if (!markedRef.current.has(m.path)) flyToOrbit(kindColor(m.kind));
    setMarked((prev) => {
      const next = new Map(prev);
      if (next.has(m.path)) next.delete(m.path);
      else {
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

  // Deletions made outside the map still land in the history and refresh the map's totals.
  const recordDeleted = (r: DeleteReport) => {
    const rep = createReport([], "trash", r.freed, r.failed.length, r.deleted.length);
    setHistory((h) => [rep, ...h].slice(0, 12));
    setNotice(`${formatBytes(r.freed)} enviados à Lixeira.`);
    scan.refresh();
  };

  const exclude = (id: string) => {
    const wasExcluded = settings.excluded.includes(id);
    const updated = wasExcluded ? settings.excluded.filter((x) => x !== id) : [...settings.excluded, id];
    setSettings({ ...settings, excluded: updated });
    setSelected((s) => s.filter((x) => x !== id));
    setNotice(
      wasExcluded
        ? "Exclusão removida. O item volta a ser considerado."
        : "Item excluído das próximas limpezas recomendadas.",
    );
  };

  const picked = useMemo(
    () => rules.filter((r) => selected.includes(r.id) && !r.blocked && !settings.excluded.includes(r.id)),
    [rules, selected, settings.excluded],
  );

  /**
   * One swallow for the whole Órbita: map marks go through `delete_paths` (and its guard), caches
   * through `junk_clean` by ID. The destination picked in the dialog travels to both.
   */
  const swallowOrbit = (destination: "trash" | "permanent") => {
    const plan = planOrbit(marked.keys(), queued.values());
    const batch = picked;
    const ids = batch.map((r) => r.id);
    const permanent = destination === "permanent";
    swallow(
      [...plan.paths, ...plan.links, ...plan.worktrees, ...ids.map((id) => `junk:${id}`)],
      async () => {
        if (ids.length && junkScanId.current === null) throw new Error("Analise novamente antes de limpar.");
        let freed = 0;
        let failed = 0;
        let skipped = 0;
        let removed = 0;
        let reason: string | undefined;
        const gone = new Set<string>();
        const tally = (r: { freed: number; failed: { error: string }[] }, done: string[]) => {
          for (const p of done) gone.add(p);
          freed += r.freed;
          failed += r.failed.length;
          skipped += r.failed.length;
          removed += done.length;
          reason ??= r.failed[0]?.error;
        };
        if (plan.paths.length) {
          const r = await api.deletePaths(plan.paths, permanent);
          tally(r, r.deleted);
        }
        // Dangling links and worktrees have their own commands and checks; both always use the trash.
        if (plan.links.length) {
          const r = await api.deleteBrokenLinks(plan.links);
          tally(r, r.deleted);
        }
        if (plan.worktrees.length) {
          const r = await api.deleteWorktrees(plan.worktrees);
          tally(r, r.removed);
        }
        setMarked((prev) => new Map([...prev].filter(([p]) => !gone.has(p))));
        setQueued((prev) => new Map([...prev].filter(([p]) => !gone.has(p))));
        if (ids.length && junkScanId.current !== null) {
          const r = await api.junkClean(ids, permanent, junkScanId.current);
          freed += r.freed;
          failed += r.failed;
          skipped += r.skipped;
          removed += r.removed;
          reason ??= r.reason ?? undefined;
        }
        const report = createReport(batch, destination, freed, skipped, removed);
        setHistory((h) => [report, ...h].slice(0, 12));
        if (ids.length) setPage("report");
        setNotice(
          failed
            ? `${formatBytes(freed)} processados, ${failed} item(ns) mantidos${reason ? `: ${reason}` : ""}.`
            : `${formatBytes(freed)} ${permanent ? "excluídos definitivamente" : "enviados à Lixeira"}.`,
        );
        return { freed, failed, reason };
      },
      async () => {
        // Panels showing removed rows start over, so nothing deleted stays listed.
        setEpoch((n) => n + 1);
        if (plan.paths.length && scan.root) await scan.refresh();
        if (ids.length) await loadJunk();
      },
    );
  };

  const addToOrbit = (items: OrbitItem[]) => {
    if (!items.length) return;
    flyToOrbit(items[0].color);
    setQueued((prev) => {
      const next = new Map(prev);
      for (const it of items) next.set(it.path, it);
      return next;
    });
  };

  const orbitRecommended = () => {
    const rec = recommended(rules, settings).map((r) => r.id);
    if (rec.some((id) => !selected.includes(id))) flyToOrbit("#ffae4a");
    setSelected((s) => [...new Set([...s, ...rec])]);
  };

  const commands: PaletteCommand[] = [
    ...PAGE_COMMANDS.map((c) => ({ ...c, run: () => setPage(c.page) })),
    { id: "scan-junk", label: "Analisar caches de novo", hint: "ação", keywords: "varrer escanear", run: loadJunk },
    ...(home
      ? [
          {
            id: "map-home",
            label: "Mapear pasta pessoal",
            hint: "ação",
            keywords: "scan home",
            run: () => {
              setMarked(new Map());
              scan.scan(home);
              setPage("map");
            },
          },
        ]
      : []),
    ...disks.map((d) => ({
      id: `map-${d.mount}`,
      label: `Mapear disco ${d.mount}`,
      hint: "ação",
      keywords: "unidade drive",
      run: () => {
        setMarked(new Map());
        scan.scan(d.mount);
        setPage("map");
      },
    })),
    { id: "orbit-rec", label: "Pôr recomendados na Órbita", hint: "ação", keywords: "caches", run: orbitRecommended },
    {
      id: "orbit-swallow",
      label: "Engolir a Órbita",
      hint: "ação",
      keywords: "excluir apagar limpar",
      run: () => {
        if (marked.size || queued.size || picked.length) setConfirm(true);
      },
    },
    {
      id: "motion",
      label: `Animações: trocar (agora ${MOTION_LABELS[motion]})`,
      hint: "preferência",
      run: cycleMotion,
    },
  ];

  const primaryDisk = disks.find((d) => d.mount === diskMount) ?? disks[0] ?? null;
  const reclaimableAmount = totalSize(recommended(rules, settings));
  // The routine is local: the app opens, looks at the history and nudges. No background service.
  const cleanupDue = settings.reminder && isCleanupDue(settings, history);

  const orbitMarks = [...marked.values()];
  const otherSize = orbitMarks.reduce((n, m) => n + m.size, 0) + [...queued.values()].reduce((n, q) => n + q.size, 0);
  const space = spaceOf(page);

  return (
    <div className="shell">
      <a
        href="#main"
        className="sr-only focus:not-sr-only focus:fixed focus:top-3 focus:left-3 focus:z-50 focus:rounded-lg focus:bg-amber-500 focus:px-4 focus:py-2 focus:font-semibold focus:text-slate-950"
      >
        Pular para o conteúdo
      </a>
      <Rail
        space={space.id}
        badges={{ limpar: scanned ? picked.length : 0 }}
        dots={{ ajustes: updateReady }}
        onSpace={goSpace}
        onPalette={() => setPalette(true)}
        motionLabel={MOTION_LABELS[motion]}
        onMotion={cycleMotion}
      />
      <div className="flex min-w-0 flex-1 flex-col overflow-hidden">
        <header className="stage-head">
          <div className="flex min-w-0 items-baseline gap-3">
            <h1 className="font-display text-2xl font-extrabold tracking-tight text-white">{space.label}</h1>
            <span className="hidden truncate text-xs text-slate-500 sm:inline">
              {platformName()} · protegido · você decide o que sai
            </span>
          </div>
          <div className="flex flex-wrap items-center gap-3">
            <SpaceTabs space={space} page={page} onPage={setPage} />
            <button type="button" className="kbtn" onClick={() => setPalette(true)}>
              <Icon name="search" size={15} />
              <span>Buscar</span>
              <kbd className="kbd">Ctrl K</kbd>
            </button>
          </div>
        </header>

        <main id="main" ref={main} tabIndex={-1} className="stage">
          <div
            className={`mx-auto flex w-full max-w-7xl flex-1 flex-col p-6 sm:p-8 ${page === "map" ? "max-w-none p-0 sm:p-0" : ""}`}
          >
            {storageError && (
              <div
                className="mb-6 flex items-center gap-3 rounded-2xl border border-amber-500/30 bg-amber-500/10 p-4 text-sm text-amber-200"
                role="alert"
              >
                <Icon name="file" size={20} />
                <span>
                  O armazenamento local não pôde ser gravado. Suas preferências não serão persistidas nesta sessão.
                </span>
              </div>
            )}
            {scanError && (
              <div
                className="mb-6 flex flex-wrap items-center justify-between gap-4 rounded-2xl border border-rose-500/30 bg-rose-500/10 p-4 text-sm text-rose-200"
                role="alert"
              >
                <div className="flex items-center gap-3">
                  <Icon name="close" size={20} />
                  <span>
                    <strong>Não foi possível concluir a análise de caches.</strong> Tente novamente.
                  </span>
                </div>
                <button
                  type="button"
                  className="cursor-pointer rounded-xl border border-rose-500/40 bg-rose-500/20 px-4 py-2 text-xs font-semibold text-rose-100 hover:bg-rose-500/30"
                  onClick={loadJunk}
                >
                  Tentar de novo
                </button>
              </div>
            )}

            {page === "overview" && (
              <Overview
                disk={primaryDisk}
                disks={disks}
                onSelectDisk={setDiskMount}
                onMapDisk={(mount) => {
                  setMarked(new Map());
                  scan.scan(mount);
                  setPage("map");
                }}
                reclaimable={reclaimableAmount}
                scanned={scanned}
                loading={junkLoading}
                due={cleanupDue}
                onScan={loadJunk}
                onCancel={() => {
                  junkTicket.current += 1;
                  setJunkLoading(false);
                }}
                navigate={setPage}
                rules={rules}
                onOrbit={orbitRecommended}
                root={scan.root}
              />
            )}

            <DiskMap
              active={page === "map"}
              scan={scan}
              home={home}
              marked={marked}
              onToggleMark={toggleMark}
              onClearMarked={() => setMarked(new Map())}
            />

            {page === "bigfiles" && (
              <BigFiles
                key={epoch}
                swallow={swallow}
                root={scan.root}
                onAnalyze={() => setPage("map")}
                onDeleted={recordDeleted}
                onOrbit={addToOrbit}
              />
            )}

            {page === "growth" && <Growth root={scan.root} onAnalyze={() => setPage("map")} />}

            {page === "review" && (
              <Review
                rules={rules}
                scanned={scanned}
                selected={selected}
                excluded={settings.excluded}
                detail={detail}
                onSelect={(id) => {
                  if (!selected.includes(id)) flyToOrbit("#ffae4a");
                  setSelected((s) => (s.includes(id) ? s.filter((x) => x !== id) : [...s, id]));
                }}
                onDetail={setDetail}
                onExclude={exclude}
                onConfirm={() => setConfirm(true)}
                onScan={loadJunk}
                loading={junkLoading}
              />
            )}

            {page === "report" && <ReportPanel report={history[0] ?? null} history={history} navigate={setPage} />}

            {page === "dupes" && (
              <DupesPanel key={epoch} swallow={swallow} defaultRoot={scan.root || home} onOrbit={addToOrbit} />
            )}
            {page === "empty" && (
              <EmptyPanel key={epoch} swallow={swallow} defaultRoot={scan.root || home} onOrbit={addToOrbit} />
            )}
            {page === "worktrees" && (
              <WorktreesPanel key={epoch} swallow={swallow} defaultRoot={scan.root || home} onOrbit={addToOrbit} />
            )}

            {page === "coverage" && <Coverage rules={rules} excluded={settings.excluded} onExclude={exclude} />}

            {page === "routine" && (
              <Routine
                settings={settings}
                rules={rules}
                history={history}
                update={setSettings}
                storageError={storageError}
              />
            )}

            {page === "updates" && <Updates />}
          </div>
        </main>
      </div>

      <Orbit
        marks={orbitMarks}
        junk={scanned ? picked : []}
        eating={hole !== null}
        onUnmark={unmark}
        onUnpick={(id) => setSelected((s) => s.filter((x) => x !== id))}
        queued={[...queued.values()]}
        onUnqueue={(path) =>
          setQueued((prev) => {
            const next = new Map(prev);
            next.delete(path);
            return next;
          })
        }
        onClear={() => {
          setMarked(new Map());
          setQueued(new Map());
          setSelected([]);
        }}
        onSwallow={() => setConfirm(true)}
      />

      {palette && <CommandPalette commands={commands} onClose={() => setPalette(false)} />}

      <div
        className={`fixed bottom-6 left-1/2 -translate-x-1/2 z-50 flex items-center gap-3 rounded-2xl border border-slate-700/80 bg-slate-900/95 px-5 py-3 text-sm font-medium text-slate-100 shadow-2xl backdrop-blur-xl transition-all duration-300 ${
          notice ? "translate-y-0 opacity-100" : "pointer-events-none translate-y-4 opacity-0"
        }`}
        role="status"
      >
        {notice && (
          <>
            <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-emerald-500/20 text-emerald-400">
              <Icon name="check" size={14} />
            </span>
            <span>{notice}</span>
            <button
              type="button"
              aria-label="Dispensar aviso"
              onClick={() => setNotice("")}
              className="ml-2 cursor-pointer text-slate-400 hover:text-white"
            >
              <Icon name="close" size={15} />
            </button>
          </>
        )}
      </div>

      {confirm && (
        <ConfirmDialog
          amount={totalSize(picked) + otherSize}
          caches={picked.length}
          marks={{ count: orbitMarks.length + queued.size, size: otherSize }}
          trashOnly={[...queued.values()].filter((q) => q.route !== "path").length}
          onPreview={() => {
            if (junkScanId.current === null) throw new Error("Analise novamente antes de simular.");
            return api.junkPreview(
              picked.map((r) => r.id),
              junkScanId.current,
            );
          }}
          onClose={() => setConfirm(false)}
          onConfirm={(destination) => {
            setConfirm(false);
            swallowOrbit(destination);
          }}
        />
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
