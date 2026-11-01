import type { Page } from "../domain";
import { gb } from "../domain";
import type { DiskInfo } from "../lib/api";
import { platformName } from "../lib/platform";
import CountUp from "./CountUp";
import Icon from "./Icon";

export default function Overview({
  disk,
  disks,
  onSelectDisk,
  onMapDisk,
  reclaimable,
  scanned,
  loading,
  due,
  onScan,
  onCancel,
  navigate,
}: {
  disk: DiskInfo | null;
  disks: DiskInfo[];
  onSelectDisk: (mount: string) => void;
  onMapDisk: (mount: string) => void;
  reclaimable: number;
  scanned: boolean;
  loading: boolean;
  due: boolean;
  onScan: () => void;
  onCancel: () => void;
  navigate: (page: Page) => void;
}) {
  // Without a reading, show placeholders rather than invented capacity.
  const known = !!disk;
  const total = disk?.total ?? 0;
  const free = disk?.free ?? 0;
  const used = Math.max(0, total - free);
  const usedPct = Math.round((used / (total || 1)) * 100);
  const freePct = 100 - usedPct;

  const circumference = 2 * Math.PI * 72;
  const strokeDashoffset = known ? circumference - (freePct / 100) * circumference : circumference;

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <p className="text-[11px] font-bold tracking-widest text-amber-400 uppercase">
            Um pouco mais de espaço. Muito mais clareza.
          </p>
          <h1 className="mt-1 text-3xl font-extrabold tracking-tight text-white sm:text-4xl">
            Seu computador,{" "}
            <span className="bg-gradient-to-r from-amber-400 via-amber-300 to-amber-500 bg-clip-text text-transparent">
              mais leve.
            </span>
          </h1>
          <p className="mt-1 text-sm text-slate-400">Entenda o que ocupa espaço. Decida o que fica.</p>
        </div>
        <div className="flex items-center gap-2.5 rounded-2xl border border-slate-800/80 bg-slate-900/80 px-4 py-2 text-xs font-medium text-slate-300 shadow-xs backdrop-blur-sm">
          <span className="relative flex h-2 w-2">
            <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-400 opacity-75" />
            <span className="relative inline-flex h-2 w-2 rounded-full bg-emerald-500" />
          </span>
          <span>Meu computador</span>
          <span className="text-slate-400">·</span>
          <span className="text-slate-400">{platformName()}</span>
          <span className="text-slate-400">·</span>
          <span className="font-semibold text-emerald-400">Protegido</span>
        </div>
      </div>

      {due && (
        <div
          className="flex flex-wrap items-center justify-between gap-4 rounded-2xl border border-amber-500/30 bg-amber-500/10 p-4 text-sm text-amber-200"
          role="status"
        >
          <div className="flex items-center gap-3">
            <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-xl bg-amber-500/20 text-amber-400">
              <Icon name="clock" size={18} />
            </span>
            <span>
              <strong>Sua limpeza de rotina está na hora.</strong> O lembrete que você configurou em Minha rotina pediu
              uma revisão.
            </span>
          </div>
          <button
            type="button"
            className="cursor-pointer rounded-xl border border-amber-500/40 bg-amber-500/20 px-4 py-2 text-xs font-semibold text-amber-200 hover:bg-amber-500/30"
            onClick={() => navigate("routine")}
          >
            Ver rotina
          </button>
        </div>
      )}

      <section
        className="grain aurora relative flex flex-col items-center gap-10 overflow-hidden rounded-3xl border border-slate-800/80 bg-gradient-to-br from-slate-900/90 via-[#0d1424] to-slate-950 p-8 shadow-2xl backdrop-blur-xl sm:p-10 md:flex-row md:items-stretch"
        aria-label="Diagnóstico do disco"
      >
        <div className="flex shrink-0 flex-col items-center justify-center">
          <div className="relative flex h-52 w-52 items-center justify-center">
            <svg className="h-full w-full -rotate-90 transform" viewBox="0 0 160 160" aria-hidden="true">
              <circle cx="80" cy="80" r="72" className="stroke-slate-800/80" strokeWidth="8" fill="none" />
              <circle
                cx="80"
                cy="80"
                r="72"
                className="stroke-amber-400 transition-all duration-1000 ease-out"
                strokeWidth="8"
                strokeDasharray={circumference}
                strokeDashoffset={strokeDashoffset}
                strokeLinecap="round"
                fill="none"
              />
            </svg>
            <div className="absolute flex flex-col items-center justify-center text-center">
              <span className="text-[10px] font-bold tracking-widest text-slate-400 uppercase">
                DISCO {disk?.mount ? `· ${disk.mount}` : "LOCAL"}
              </span>
              <span className="mt-1 font-mono text-3xl font-extrabold tracking-tight text-white sm:text-4xl">
                {known ? <CountUp value={free} format={gb} /> : "—"}
              </span>
              <span className="mt-0.5 text-xs text-slate-400">
                {known ? `livres de ${gb(total)}` : "lendo o disco…"}
              </span>
            </div>
          </div>
          <div className="mt-3 flex items-center gap-2 text-xs text-slate-400">
            <span className="h-2 w-2 rounded-full bg-amber-400 shadow-[0_0_8px_rgba(251,191,36,0.6)]" />
            <span>Espaço livre no disco</span>
          </div>
        </div>

        <div className="flex flex-1 flex-col justify-between">
          <div>
            <span className="inline-flex items-center gap-1.5 rounded-full border border-amber-500/25 bg-amber-500/10 px-3 py-1 text-xs font-semibold tracking-wide text-amber-300 uppercase">
              <Icon name="spark" size={13} />
              <span>Limpeza com critério</span>
            </span>
            <h2 className="mt-3 text-2xl font-bold tracking-tight text-white sm:text-3xl">
              {loading
                ? "Olhando cada cantinho…"
                : scanned
                  ? reclaimable === 0
                    ? "Tudo em ordem por aqui."
                    : "Tem espaço para respirar."
                  : "Vamos encontrar o que sobra?"}
            </h2>
            <p className="mt-2 text-sm leading-relaxed text-slate-300">
              {loading
                ? "A análise de caches vasculha apenas pastas descartáveis com segurança."
                : scanned
                  ? reclaimable === 0
                    ? "Nenhum arquivo desnecessário detectado neste momento."
                    : "Encontramos arquivos temporários e caches. Você revisa tudo antes de limpar."
                  : "Uma análise rápida encontra caches e temporários. Seus arquivos pessoais continuam no lugar."}
            </p>
            <div className="mt-5 flex flex-col">
              <div className="font-mono text-5xl font-extrabold tracking-tight text-amber-400 sm:text-6xl">
                {loading ? "…" : scanned ? <CountUp value={reclaimable} format={gb} /> : "—"}
              </div>
              <span className="mt-1 text-xs font-semibold tracking-wider text-slate-400 uppercase">
                {loading
                  ? "analisando caches do sistema"
                  : scanned
                    ? "para revisar · tamanho estimado"
                    : "sua próxima análise começa aqui"}
              </span>
            </div>
          </div>

          <div className="mt-6 flex flex-col gap-4">
            {loading ? (
              <div className="flex items-center gap-3">
                <button
                  type="button"
                  className="cursor-pointer rounded-xl border border-slate-700 bg-slate-800/80 px-5 py-3 text-sm font-semibold text-slate-200 transition-colors hover:bg-slate-700"
                  onClick={onCancel}
                >
                  Cancelar análise
                </button>
              </div>
            ) : (
              <div className="flex flex-wrap items-center gap-3">
                <button
                  type="button"
                  className={`glow-button inline-flex cursor-pointer items-center justify-center gap-2 rounded-xl bg-gradient-to-r from-amber-500 via-amber-400 to-amber-500 px-6 py-3.5 text-sm font-bold text-slate-950 shadow-xl shadow-amber-500/20 transition-all hover:brightness-110 active:scale-[0.98] ${
                    scanned && reclaimable > 0 ? "" : ""
                  }`}
                  onClick={scanned && reclaimable > 0 ? () => navigate("review") : onScan}
                >
                  <Icon name={scanned && reclaimable > 0 ? "arrow" : "search"} size={16} />
                  <span>{scanned && reclaimable > 0 ? "Revisar limpeza" : "Analisar meu computador"}</span>
                </button>
                {scanned && (
                  <button
                    type="button"
                    className="inline-flex cursor-pointer items-center gap-2 rounded-xl border border-slate-700/80 bg-slate-850 px-4 py-3 text-sm font-semibold text-slate-200 transition-all hover:border-slate-600 hover:bg-slate-800 active:scale-[0.98]"
                    onClick={onScan}
                  >
                    <Icon name="refresh" size={16} />
                    <span>Analisar novamente</span>
                  </button>
                )}
              </div>
            )}
            <div className="flex items-center gap-2 text-xs text-slate-400">
              <Icon name="shield" size={15} />
              <span>Você decide. Proteção total a arquivos de trabalho.</span>
            </div>
          </div>
        </div>
      </section>

      <div className="grid grid-cols-1 gap-6 md:grid-cols-2">
        <section className="flex flex-col justify-between gap-4 rounded-2xl border border-slate-800/80 bg-slate-900/60 p-6 shadow-lg backdrop-blur-md">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <h2 className="text-base font-bold text-white">Um olhar sobre o disco</h2>
            <div className="flex flex-wrap items-center gap-2 text-xs text-slate-400">
              {disks.length > 1 && disk ? (
                <select
                  aria-label="Escolher unidade de disco"
                  value={disk.mount}
                  onChange={(e) => onSelectDisk(e.target.value)}
                  className="rounded-lg border border-slate-700 bg-slate-800 px-2.5 py-1 text-xs text-slate-200 outline-none focus:border-amber-500"
                >
                  {disks.map((d) => (
                    <option key={d.mount} value={d.mount}>
                      Unidade {d.mount} ({gb(d.free)} livres de {gb(d.total)})
                    </option>
                  ))}
                </select>
              ) : (
                disk?.mount && <span>Unidade {disk.mount} · </span>
              )}
              <span className="font-mono text-slate-300">{known ? `${gb(total)} total` : "—"}</span>
            </div>
          </div>

          {known && (
            <div
              className="flex h-4 w-full overflow-hidden rounded-full border border-slate-700/50 bg-slate-800/90 p-0.5"
              role="img"
              aria-label={`Disco: ${gb(used)} ocupados (${usedPct}%), ${gb(free)} livres (${freePct}%)`}
            >
              <div
                className="h-full rounded-full bg-gradient-to-r from-blue-500 to-indigo-500 transition-all duration-500"
                style={{ width: `${usedPct}%` }}
              />
              <div
                className="h-full rounded-full bg-slate-900/50 transition-all duration-500"
                style={{ width: `${freePct}%` }}
              />
            </div>
          )}

          <div className="grid grid-cols-1 gap-2 text-xs sm:grid-cols-3">
            <div className="flex items-center gap-2 rounded-xl border border-slate-800/60 bg-slate-950/40 p-2.5">
              <span className="h-2 w-2 shrink-0 rounded-full bg-blue-500" />
              <div className="flex flex-col">
                <span className="text-[10px] text-slate-400">Ocupado</span>
                <strong className="font-mono text-slate-200">{known ? gb(used) : "—"}</strong>
              </div>
            </div>
            <div className="flex items-center gap-2 rounded-xl border border-slate-800/60 bg-slate-950/40 p-2.5">
              <span className="h-2 w-2 shrink-0 rounded-full bg-slate-600" />
              <div className="flex flex-col">
                <span className="text-[10px] text-slate-400">Livre</span>
                <strong className="font-mono text-slate-200">{known ? gb(free) : "—"}</strong>
              </div>
            </div>
            <div className="flex items-center gap-2 rounded-xl border border-amber-500/20 bg-amber-500/5 p-2.5">
              <span className="h-2 w-2 shrink-0 rounded-full bg-amber-400 shadow-[0_0_6px_rgba(251,191,36,0.6)]" />
              <div className="flex flex-col">
                <span className="text-[10px] text-amber-300">Caches revisáveis</span>
                <strong className="font-mono text-amber-300">{gb(reclaimable)}</strong>
              </div>
            </div>
          </div>

          {disk && (
            <button
              type="button"
              className="mt-1 inline-flex cursor-pointer items-center gap-2 text-xs font-semibold text-amber-400 hover:text-amber-300"
              onClick={() => onMapDisk(disk.mount)}
            >
              <Icon name="grid" size={15} />
              <span>Mapear esta unidade visualmente</span>
            </button>
          )}
        </section>

        <section className="flex items-start gap-4 rounded-2xl border border-emerald-500/20 bg-gradient-to-br from-emerald-950/20 via-slate-900/60 to-slate-900/60 p-6 shadow-lg backdrop-blur-md transition-colors hover:border-emerald-500/35">
          <span className="flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl border border-emerald-500/20 bg-emerald-500/10 text-emerald-400 shadow-sm">
            <Icon name="shield" size={24} />
          </span>
          <div className="flex flex-col">
            <h2 className="text-base font-bold text-white">Limpar sem perder o que importa</h2>
            <p className="mt-1 text-sm leading-relaxed text-slate-400">
              Veja exatamente o que protegemos por padrão e personalize suas preferências e exclusões.
            </p>
            <button
              type="button"
              className="mt-4 inline-flex w-fit cursor-pointer items-center gap-2 rounded-xl border border-emerald-500/30 bg-emerald-500/10 px-4 py-2 text-xs font-semibold text-emerald-300 transition-all hover:bg-emerald-500/20 active:scale-[0.98]"
              onClick={() => navigate("review")}
            >
              <span>Conhecer as proteções</span>
              <Icon name="arrow" size={14} />
            </button>
          </div>
        </section>
      </div>

      <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
        {[
          "Sem mexer em senhas ou histórico",
          "Sem encerrar seus aplicativos",
          "Cada decisão explicada com transparência",
        ].map((text) => (
          <div
            key={text}
            className="flex items-center gap-3 rounded-2xl border border-slate-800/80 bg-slate-900/40 px-4 py-3 text-xs font-medium text-slate-300 shadow-xs backdrop-blur-sm transition-colors hover:border-slate-700/80"
          >
            <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-emerald-500/20 text-emerald-400">
              <Icon name="check" size={13} />
            </span>
            <span>{text}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
