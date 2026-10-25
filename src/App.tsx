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

  const loadJunk = useCallback(async () => {
    setJunkLoading(true);
    setScanError(false);
    try {
      const items = await api.junkScan();
      const merged = mergeRulesWithScan(items);
      setRules(merged);
      setScanned(true);
      const rec = recommended(merged, settings).map((r) => r.id);
      setSelected(rec);
      const firstActive = merged.find((r) => r.size > 0 && !r.blocked) || merged[0];
      setDetail(firstActive);
      setNotice(
        rec.length ? "Análise concluída. Revise os itens para liberar espaço." : "Análise concluída: tudo limpo!",
      );
    } catch {
      setScanError(true);
    } finally {
      setJunkLoading(false);
    }
  }, [settings]);

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
        const r = await api.junkClean(ids, destination === "permanent");
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

  const primaryDisk = disks[0] ?? null;
  const reclaimableAmount = totalSize(recommended(rules, settings));
  // The routine is local: the app opens, looks at the history and nudges. No background service.
  const cleanupDue = settings.reminder && isCleanupDue(settings, history);

  return (
    <div className="app-shell">
      <a href="#main" className="skip-link">
        Pular para o conteúdo
      </a>
      <aside className="sidebar">
        <button type="button" className="brand" onClick={() => setPage("overview")}>
          <span className="brand-mark">
            <i />
            <i />
            <i />
            <i />
          </span>
          faxina<span className="brand-period">.</span>
        </button>
        <span className="sidebar-tagline">Espaço para o que importa.</span>
        <nav aria-label="Navegação principal" className="sidebar-nav">
          {NAV.map((n) => (
            <div key={n.id}>
              {n.caption && <span className="nav-caption">{n.caption}</span>}
              <button
                type="button"
                className={`nav-link ${page === n.id ? "active" : ""}`}
                aria-current={page === n.id ? "page" : undefined}
                onClick={() => setPage(n.id)}
              >
                <Icon name={n.icon} />
                <span>{n.label}</span>
                {n.id === "review" && scanned && picked.length > 0 && (
                  <span className="nav-count">{picked.length}</span>
                )}
              </button>
            </div>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <button
            type="button"
            onClick={cycleMotion}
            title="Animação de exclusão: seguir o sistema, sempre ou nunca"
            className="motion-button"
          >
            <span aria-hidden="true">✨</span>
            <span>Animações: {MOTION_LABELS[motion]}</span>
          </button>
          <div className="privacy-card">
            <Icon name="shield" size={22} />
            <strong>Seu espaço é seu.</strong>
            <p>
              Você escolhe o que limpar.
              <br />
              Seus dados ficam com você.
            </p>
          </div>
          <span className="prototype-label">
            <span className="status-dot" />
            PROTEÇÃO ATIVA
          </span>
        </div>
      </aside>
      <div className="workspace">
        <header className="topbar">
          <span>
            Meu computador <span className="breadcrumb-separator">/</span>{" "}
            <strong>{NAV.find((n) => n.id === page)?.label}</strong>
          </span>
          <div className="flex items-center gap-3">
            <button
              type="button"
              onClick={cycleMotion}
              title="Animação de exclusão: seguir o sistema, sempre ou nunca"
              className="flex items-center gap-1.5 rounded-md border border-[var(--color-line)] bg-[var(--color-panel)] px-2.5 py-1 text-xs text-[var(--color-muted)] hover:text-white"
            >
              <span aria-hidden="true">✨</span>
              <span>Animações: {MOTION_LABELS[motion]}</span>
            </button>
            <div className="topbar-status">
              <Icon name="shield" size={15} />
              Protegido · {platformName()}
            </div>
          </div>
        </header>

        <main id="main" ref={main} tabIndex={-1}>
          <div className={`content ${page === "map" ? "map-page" : ""}`}>
            {storageError && (
              <div className="callout amber-callout" role="alert">
                O armazenamento local não pôde ser gravado. Suas preferências não serão persistidas nesta sessão.
              </div>
            )}
            {scanError && (
              <div className="callout error-callout" role="alert">
                <Icon name="close" />
                <span>
                  <strong>Não foi possível concluir a análise de caches.</strong> Tente novamente.
                </span>
                <button type="button" className="button secondary" onClick={loadJunk}>
                  Tentar de novo
                </button>
              </div>
            )}

            {page === "overview" && (
              <Overview
                disk={primaryDisk}
                reclaimable={reclaimableAmount}
                scanned={scanned}
                loading={junkLoading}
                due={cleanupDue}
                onScan={loadJunk}
                onCancel={() => setJunkLoading(false)}
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

            <footer className="page-footer">
              <span>Feito para liberar espaço. Pensado para preservar o que importa.</span>
              <span>faxina · desktop cleaner</span>
            </footer>
          </div>
        </main>
      </div>

      <div className={`toast ${notice ? "visible" : ""}`} role="status">
        {notice && (
          <>
            <Icon name="check" size={16} />
            <span>{notice}</span>
            <button type="button" aria-label="Dispensar aviso" onClick={() => setNotice("")}>
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
