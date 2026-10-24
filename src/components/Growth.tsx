import { useEffect, useState } from "react";
import { api, type Growth as GrowthData } from "../lib/api";
import { formatAge, formatBytes } from "../lib/format";
import Icon from "./Icon";

export default function Growth({ root, onAnalyze }: { root: string; onAnalyze: () => void }) {
  const [growth, setGrowth] = useState<GrowthData | null>(null);
  const [loaded, setLoaded] = useState(false);

  // biome-ignore lint/correctness/useExhaustiveDependencies: a new scan root means a new comparison
  useEffect(() => {
    setLoaded(false);
    api.growth().then(
      (g) => {
        setGrowth(g);
        setLoaded(true);
      },
      () => {
        setGrowth(null);
        setLoaded(true);
      },
    );
  }, [root]);

  const maxGrowth = growth?.grown[0]?.growth ?? 1;
  const grew = (growth?.delta ?? 0) >= 0;

  return (
    <>
      <div className="page-heading compact">
        <div>
          <p className="eyebrow">COMPARAÇÃO ENTRE ANÁLISES</p>
          <h1>O que cresceu desde a última vez.</h1>
          <p>Cada análise do Mapa guarda os tamanhos das pastas grandes, só neste computador.</p>
        </div>
        <span className="badge">Somente leitura</span>
      </div>
      {!root || (loaded && !growth) ? (
        <section className="panel empty-state">
          <Icon name="clock" size={38} />
          <h2>{root ? "Primeira análise registrada." : "Nenhuma análise ainda."}</h2>
          <p>
            {root
              ? "Analise a mesma pasta de novo mais tarde para ver o que cresceu entre as duas."
              : "Analise uma pasta no Mapa do disco. Na próxima análise da mesma pasta, a comparação aparece aqui."}
          </p>
          <button type="button" className="button primary" onClick={onAnalyze}>
            Ir para o Mapa do disco
          </button>
        </section>
      ) : (
        growth && (
          <>
            <div className="metric-grid">
              <div className="panel metric emphasized">
                <span>{grew ? "Cresceu" : "Diminuiu"}</span>
                <strong>
                  {grew ? "+" : "−"}
                  {formatBytes(Math.abs(growth.delta))}
                </strong>
                <small>Comparado à análise {formatAge(growth.previousAt)}</small>
              </div>
            </div>
            <section className="panel history-panel">
              <div className="section-heading">
                <h2>Pastas que mais cresceram</h2>
                <span className="muted" title={growth.root}>
                  {growth.root}
                </span>
              </div>
              {growth.grown.length ? (
                <ul className="flex flex-col gap-3">
                  {growth.grown.map((f) => (
                    <li key={f.path}>
                      <div className="flex items-baseline justify-between gap-4">
                        <span className="truncate" title={f.path}>
                          {f.path}
                        </span>
                        <strong className="tabular-nums">+{formatBytes(f.growth)}</strong>
                      </div>
                      <div className="mt-1 h-1.5 rounded bg-[var(--color-panel-2)]">
                        <div
                          className="h-full rounded bg-[var(--color-accent)]"
                          style={{ width: `${Math.max(2, (f.growth / maxGrowth) * 100)}%` }}
                        />
                      </div>
                      <small className="muted">
                        {f.beforeKnown ? formatBytes(f.before) : `menos de ${formatBytes(f.before)}`} →{" "}
                        {formatBytes(f.after)}
                      </small>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="muted">Nenhuma pasta cresceu de forma relevante (mais de 100 MB).</p>
              )}
            </section>
            {!grew && (
              <div className="callout amber-callout">
                <Icon name="file" />
                <span>Se alguma pasta não pôde ser lida nesta análise, parte da redução pode não ser real.</span>
              </div>
            )}
          </>
        )
      )}
    </>
  );
}
