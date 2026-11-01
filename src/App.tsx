import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { BigFiles } from "./components/BigFiles";
import { BlackHole } from "./components/BlackHole";
import ConfirmDialog from "./components/ConfirmDialog";
import Coverage from "./components/Coverage";
import DiskMap from "./components/DiskMap";
import { DupesPanel } from "./components/DupesPanel";
import { EmptyPanel } from "./components/EmptyPanel";
import Growth from "./components/Growth";
import Icon, { type IconName } from "./components/Icon";
import Overview from "./components/Overview";
import ReportPanel from "./components/ReportPanel";
import Review from "./components/Review";
import Routine from "./components/Routine";
import Updates from "./components/Updates";
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
import { formatBytes } from "./lib/format";
import type { Mark } from "./lib/marks";
import { getMotionPref, MOTION_LABELS, type MotionPref, setMotionPref } from "./lib/motion";
import { platformName } from "./lib/platform";

const NAV: { id: Page; label: string; icon: IconName; caption?: string }[] = [
  { id: "overview", label: "Visão geral", icon: "grid" },
  { id: "map", label: "Mapa do disco", icon: "grid" },
  { id: "bigfiles", label: "Arquivos grandes", icon: "file" },
  { id: "growth", label: "O que cresceu", icon: "clock" },
  { id: "review", label: "Revisar limpeza", icon: "spark" },
  { id: "report", label: "Resultados", icon: "file" },
  { id: "dupes", label: "Duplicatas", icon: "copy" },
  { id: "empty", label: "Vazios e links", icon: "trash" },
  { id: "coverage", label: "Aplicativos", icon: "box", caption: "PERSONALIZAR" },
  { id: "routine", label: "Minha rotina", icon: "clock" },
  { id: "updates", label: "Atualizações", icon: "download" },
];

export default function App() {
  const [page, setPage] = useState<Page>("overview");
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
  const [motion, setMotion] = useState<MotionPref>(getMotionPref);

  const scan = useScan();
  const { hole, swallow, finish } = useHole();
  const main = useRef<HTMLElement>(null);

  const cycleMotion = () => {
    const next: MotionPref = motion === "system" ? "always" : motion === "always" ? "never" : "system";
    setMotionPref(next);
    setMotion(next);
  };

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

  const toggleMark = useCallback((m: Mark) => {
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

  const deleteMarked = (permanent: boolean) => {
    const paths = [...marked.keys()];
    swallow(
      paths,
      async () => {
        const r = await api.deletePaths(paths, permanent);
        setMarked((prev) => new Map([...prev].filter(([p]) => !r.deleted.includes(p))));
        const rep = createReport([], permanent ? "permanent" : "trash", r.freed, r.failed.length, r.deleted.length);
        setHistory((h) => [rep, ...h].slice(0, 12));
        setNotice(`${formatBytes(r.freed)} ${permanent ? "excluídos definitivamente" : "enviados à Lixeira"}.`);
        return { freed: r.freed, failed: r.failed.length, reason: r.failed[0]?.error };
      },
      scan.refresh,
    );
  };

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

  const cleanJunk = (destination: "trash" | "permanent") => {
    const ids = picked.map((r) => r.id);
    swallow(
      ids.map((id) => `junk:${id}`),
      async () => {
        if (junkScanId.current === null) throw new Error("Analise novamente antes de limpar.");
        const r = await api.junkClean(ids, destination === "permanent", junkScanId.current);
        const report = createReport(picked, destination, r.freed, r.skipped, r.removed);
        setHistory((h) => [report, ...h].slice(0, 12));
        setPage("report");
        setNotice(
          r.failed
            ? `${formatBytes(r.freed)} processados, ${r.failed} item(ns) mantidos${r.reason ? `: ${r.reason}` : ""}.`
            : `Limpeza concluída: ${formatBytes(r.freed)} ${
                destination === "trash" ? "enviados à Lixeira" : "liberados"
              }.`,
        );
        return { freed: r.freed, failed: r.failed, reason: r.reason ?? undefined };
      },
      loadJunk,
    );
  };

  const primaryDisk = disks.find((d) => d.mount === diskMount) ?? disks[0] ?? null;
  const reclaimableAmount = totalSize(recommended(rules, settings));
  // The routine is local: the app opens, looks at the history and nudges. No background service.
  const cleanupDue = settings.reminder && isCleanupDue(settings, history);

  return (
    <div className="flex h-screen max-h-screen overflow-hidden bg-[var(--color-ink)] text-[var(--color-text)]">
      <a
        href="#main"
        className="sr-only focus:not-sr-only focus:fixed focus:top-3 focus:left-3 focus:z-50 focus:rounded-lg focus:bg-amber-500 focus:px-4 focus:py-2 focus:font-semibold focus:text-slate-950"
      >
        Pular para o conteúdo
      </a>
      <aside className="flex w-60 min-w-60 shrink-0 flex-col border-r border-slate-800/70 bg-[#0b101b]/95 p-4 backdrop-blur-xl">
        <button
          type="button"
          className="group flex cursor-pointer items-center gap-3 px-2 py-1 text-left transition-transform hover:scale-[1.01]"
          onClick={() => setPage("overview")}
        >
          <span className="grid grid-cols-2 gap-1 rounded-lg bg-slate-900/80 p-1.5 shadow-sm transition-transform duration-300 group-hover:rotate-6">
            <i className="h-2 w-2 rounded-xs bg-amber-400" />
            <i className="h-2 w-2 rounded-xs bg-blue-500" />
            <i className="h-2 w-2 rounded-xs bg-emerald-400" />
            <i className="h-2 w-2 rounded-xs bg-rose-400" />
          </span>
          <span className="text-xl font-bold tracking-tight text-white">
            faxina<span className="text-amber-400">.</span>
          </span>
        </button>
        <span className="mt-1 px-2 text-[11px] font-medium tracking-wide text-slate-400/80">
          Espaço para o que importa.
        </span>
        <nav aria-label="Navegação principal" className="mt-5 flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto pr-1">
          {NAV.map((n) => (
            <div key={n.id}>
              {n.caption && (
                <span className="block px-3 pt-4 pb-1 text-[10px] font-bold tracking-widest text-slate-400 uppercase">
                  {n.caption}
                </span>
              )}
              <button
                type="button"
                className={`group flex w-full cursor-pointer items-center gap-3 rounded-xl px-3 py-2 text-sm font-medium transition-all duration-150 ${
                  page === n.id
                    ? "border border-amber-500/25 bg-amber-500/10 font-semibold text-amber-300 shadow-xs"
                    : "border border-transparent text-slate-400 hover:bg-slate-800/40 hover:text-slate-200"
                }`}
                aria-current={page === n.id ? "page" : undefined}
                onClick={() => setPage(n.id)}
              >
                <Icon name={n.icon} size={18} />
                <span>{n.label}</span>
                {n.id === "review" && scanned && picked.length > 0 && (
                  <span className="ml-auto rounded-full border border-amber-500/30 bg-amber-500/20 px-2 py-0.5 font-mono text-[11px] font-bold text-amber-300 shadow-xs">
                    {picked.length}
                  </span>
                )}
              </button>
            </div>
          ))}
        </nav>
        <div className="mt-auto flex shrink-0 flex-col gap-3 pt-3 border-t border-slate-800/60">
          <button
            type="button"
            onClick={cycleMotion}
            title="Animação de exclusão: seguir o sistema, sempre ou nunca"
            className="flex w-full cursor-pointer items-center justify-between rounded-xl border border-slate-800 bg-slate-900/60 px-3 py-2 text-xs text-slate-300 shadow-xs transition-colors hover:bg-slate-800/70"
          >
            <span className="flex items-center gap-2">
              <span aria-hidden="true">✨</span>
              <span>Animações</span>
            </span>
            <span className="font-medium text-amber-400/90">{MOTION_LABELS[motion]}</span>
          </button>
          <div className="rounded-2xl border border-slate-800/80 bg-gradient-to-b from-slate-900/60 to-slate-950/80 p-3.5 text-xs text-slate-400 shadow-xs">
            <div className="mb-2 flex items-center gap-2 text-emerald-400">
              <Icon name="shield" size={18} />
              <strong className="text-slate-200">Seu espaço é seu.</strong>
            </div>
            <p className="leading-relaxed">
              Você escolhe o que limpar.
              <br />
              Seus dados ficam com você.
            </p>
          </div>
          <div className="flex items-center gap-2 px-1 pt-1 text-[10px] font-bold tracking-wider text-slate-400 uppercase">
            <span className="relative flex h-2 w-2">
              <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-400 opacity-75" />
              <span className="relative inline-flex h-2 w-2 rounded-full bg-emerald-500" />
            </span>
            <span>PROTEÇÃO ATIVA</span>
          </div>
        </div>
      </aside>
      <div className="flex min-w-0 flex-1 flex-col overflow-hidden">
        <header className="sticky top-0 z-10 flex h-16 shrink-0 items-center justify-between border-b border-slate-800/70 bg-[#0b101b]/80 px-8 text-xs text-slate-400 backdrop-blur-md">
          <div className="flex items-center gap-2">
            <span>Meu computador</span>
            <span className="text-slate-400">/</span>
            <strong className="font-semibold text-slate-100">{NAV.find((n) => n.id === page)?.label}</strong>
          </div>
          <div className="flex items-center gap-3">
            <button
              type="button"
              onClick={cycleMotion}
              title="Animação de exclusão: seguir o sistema, sempre ou nunca"
              className="flex cursor-pointer items-center gap-1.5 rounded-full border border-slate-800 bg-slate-900/80 px-3 py-1.5 text-xs text-slate-400 transition-colors hover:text-white"
            >
              <span aria-hidden="true">✨</span>
              <span>Animações: {MOTION_LABELS[motion]}</span>
            </button>
            <div className="inline-flex items-center gap-2 rounded-full border border-emerald-500/20 bg-emerald-500/10 px-3 py-1.5 text-xs font-medium text-emerald-400 shadow-xs">
              <Icon name="shield" size={14} />
              <span>Protegido · {platformName()}</span>
            </div>
          </div>
        </header>

        <main id="main" ref={main} tabIndex={-1} className="flex min-h-0 flex-1 flex-col overflow-y-auto outline-none">
          <div
            className={`mx-auto flex w-full max-w-7xl flex-1 flex-col p-8 sm:p-10 ${page === "map" ? "p-0 sm:p-0 max-w-none" : ""}`}
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
              />
            )}

            <DiskMap
              active={page === "map"}
              scan={scan}
              home={home}
              marked={marked}
              onToggleMark={toggleMark}
              onClearMarked={() => setMarked(new Map())}
              onUnmark={unmark}
              onDeleteMarked={deleteMarked}
            />

            {page === "bigfiles" && (
              <BigFiles swallow={swallow} root={scan.root} onAnalyze={() => setPage("map")} onDeleted={recordDeleted} />
            )}

            {page === "growth" && <Growth root={scan.root} onAnalyze={() => setPage("map")} />}

            {page === "review" && (
              <Review
                rules={rules}
                scanned={scanned}
                selected={selected}
                excluded={settings.excluded}
                detail={detail}
                onSelect={(id) => setSelected((s) => (s.includes(id) ? s.filter((x) => x !== id) : [...s, id]))}
                onDetail={setDetail}
                onExclude={exclude}
                onConfirm={() => setConfirm(true)}
                onScan={loadJunk}
                loading={junkLoading}
              />
            )}

            {page === "report" && <ReportPanel report={history[0] ?? null} history={history} navigate={setPage} />}

            {page === "dupes" && <DupesPanel swallow={swallow} defaultRoot={scan.root || home} />}
            {page === "empty" && <EmptyPanel swallow={swallow} defaultRoot={scan.root || home} />}

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

            {page !== "map" && (
              <footer className="mt-16 flex flex-wrap items-center justify-between gap-4 border-t border-slate-800/60 pt-6 text-xs text-slate-500">
                <span>Feito para liberar espaço. Pensado para preservar o que importa.</span>
                <span className="font-mono text-slate-600">faxina · desktop cleaner</span>
              </footer>
            )}
          </div>
        </main>
      </div>

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
          amount={totalSize(picked)}
          onClose={() => setConfirm(false)}
          onConfirm={(destination) => {
            setConfirm(false);
            cleanJunk(destination);
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
