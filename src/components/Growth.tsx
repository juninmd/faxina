import { useEffect, useState } from "react";
import { api, type Growth as GrowthData } from "../lib/api";
import { formatAge, formatBytes } from "../lib/format";
import Icon from "./Icon";

export default function Growth({ root, onAnalyze }: { root: string; onAnalyze: () => void }) {
  const [growth, setGrowth] = useState<GrowthData | null>(null);
  const [loaded, setLoaded] = useState(false);
  const [failed, setFailed] = useState(false);

  // biome-ignore lint/correctness/useExhaustiveDependencies: a new scan root means a new comparison
  useEffect(() => {
    // A reply for a scan root the user has already left must not replace the current one.
    let current = true;
    setLoaded(false);
    setFailed(false);
    api.growth().then(
      (g) => {
        if (!current) return;
        setGrowth(g);
        setLoaded(true);
      },
      () => {
        if (!current) return;
        setGrowth(null);
        setFailed(true);
        setLoaded(true);
      },
    );
    return () => {
      current = false;
    };
  }, [root]);

  const maxGrowth = growth?.grown[0]?.growth ?? 1;
  const grew = (growth?.delta ?? 0) >= 0;

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <p className="text-[11px] font-bold tracking-widest text-amber-400 uppercase">Comparação entre análises</p>
          <h1 className="mt-1 text-3xl font-extrabold tracking-tight text-white sm:text-4xl">
            O que cresceu desde a última vez.
          </h1>
          <p className="mt-1 text-sm text-slate-400">
            Cada análise do Mapa guarda os tamanhos das pastas grandes, só neste computador.
          </p>
        </div>
        <span className="rounded-full border border-slate-700/80 bg-slate-800/60 px-4 py-2 text-xs font-semibold text-slate-300 shadow-xs backdrop-blur-sm">
          Somente leitura
        </span>
      </div>

      {root && !loaded && <p className="text-xs text-slate-400">Carregando a comparação…</p>}

      {root && failed ? (
        <section className="flex flex-col items-center justify-center gap-4 rounded-3xl border border-slate-800/80 bg-slate-900/60 p-12 text-center shadow-lg backdrop-blur-md">
          <span className="flex h-16 w-16 items-center justify-center rounded-2xl border border-rose-500/30 bg-rose-500/15 text-rose-400">
            <Icon name="clock" size={32} />
          </span>
          <h2 className="text-xl font-bold text-white">Não foi possível comparar.</h2>
          <p className="max-w-md text-sm text-slate-400">Analise a pasta de novo para tentar outra vez.</p>
          <button
            type="button"
            className="cursor-pointer rounded-xl bg-gradient-to-r from-amber-500 to-amber-400 px-6 py-2.5 text-xs font-bold text-slate-950 shadow-lg shadow-amber-500/20 hover:brightness-110"
            onClick={onAnalyze}
          >
            Ir para o Mapa do disco
          </button>
        </section>
      ) : !root || (loaded && !growth) ? (
        <section className="flex flex-col items-center justify-center gap-4 rounded-3xl border border-slate-800/80 bg-slate-900/60 p-12 text-center shadow-lg backdrop-blur-md">
          <span className="flex h-16 w-16 items-center justify-center rounded-2xl border border-slate-700 bg-slate-800 text-amber-400">
            <Icon name="clock" size={32} />
          </span>
          <h2 className="text-xl font-bold text-white">
            {root ? "Primeira análise registrada." : "Nenhuma análise ainda."}
          </h2>
          <p className="max-w-md text-sm text-slate-400">
            {root
              ? "Analise a mesma pasta de novo mais tarde para ver o que cresceu entre as duas."
              : "Analise uma pasta no Mapa do disco. Na próxima análise da mesma pasta, a comparação aparece aqui."}
          </p>
          <button
            type="button"
            className="cursor-pointer rounded-xl bg-gradient-to-r from-amber-500 to-amber-400 px-6 py-2.5 text-xs font-bold text-slate-950 shadow-lg shadow-amber-500/20 hover:brightness-110"
            onClick={onAnalyze}
          >
            Ir para o Mapa do disco
          </button>
        </section>
      ) : (
        growth && (
          <>
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <div
                className={`flex flex-col rounded-2xl border p-6 shadow-xl backdrop-blur-md ${
                  grew
                    ? "border-amber-500/40 bg-gradient-to-br from-amber-500/10 via-slate-900 to-slate-950"
                    : "border-emerald-500/40 bg-gradient-to-br from-emerald-500/10 via-slate-900 to-slate-950"
                }`}
              >
                <span
                  className={`text-xs font-bold uppercase tracking-wider ${grew ? "text-amber-400" : "text-emerald-400"}`}
                >
                  {grew ? "Cresceu" : "Diminuiu"}
                </span>
                <strong
                  className={`mt-2 font-mono text-4xl font-extrabold ${grew ? "text-amber-400" : "text-emerald-400"}`}
                >
                  {grew ? "+" : "−"}
                  {formatBytes(Math.abs(growth.delta))}
                </strong>
                <small className="mt-1 text-xs text-slate-400">
                  Comparado à análise {formatAge(growth.previousAt)}
                </small>
              </div>
            </div>

            <section className="flex flex-col gap-4 rounded-2xl border border-slate-800/80 bg-slate-900/60 p-6 shadow-lg backdrop-blur-md">
              <div className="flex items-center justify-between pb-3 border-b border-slate-800/60">
                <h2 className="text-base font-bold text-white">Pastas que mais cresceram</h2>
                <span className="font-mono text-xs text-slate-400 truncate max-w-sm" title={growth.root}>
                  {growth.root}
                </span>
              </div>
              {growth.grown.length ? (
                <ul className="flex flex-col gap-3">
                  {growth.grown.map((f) => (
                    <li
                      key={f.path}
                      className="flex flex-col gap-1.5 rounded-xl border border-slate-800/60 bg-slate-950/40 p-3.5"
                    >
                      <div className="flex items-baseline justify-between gap-4">
                        <span className="font-mono text-xs text-slate-300 truncate" title={f.path}>
                          {f.path}
                        </span>
                        <strong className="font-mono text-xs font-bold text-amber-400 tabular-nums">
                          +{formatBytes(f.growth)}
                        </strong>
                      </div>
                      <div className="h-2 w-full overflow-hidden rounded-full bg-slate-800">
                        <div
                          className="h-full rounded-full bg-gradient-to-r from-amber-500 to-amber-400"
                          style={{ width: `${Math.max(2, (f.growth / maxGrowth) * 100)}%` }}
                        />
                      </div>
                      <small className="text-[11px] text-slate-400">
                        {f.beforeKnown ? formatBytes(f.before) : `menos de ${formatBytes(f.before)}`} →{" "}
                        {formatBytes(f.after)}
                      </small>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="text-xs text-slate-400">Nenhuma pasta cresceu de forma relevante (mais de 100 MB).</p>
              )}
            </section>

            {!grew && (
              <div className="flex items-center gap-3 rounded-2xl border border-amber-500/30 bg-amber-500/10 p-4 text-xs leading-relaxed text-amber-200">
                <Icon name="file" size={18} />
                <span>Se alguma pasta não pôde ser lida nesta análise, parte da redução pode não ser real.</span>
              </div>
            )}
          </>
        )
      )}
    </div>
  );
}
