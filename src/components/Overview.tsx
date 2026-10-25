import type { Page } from "../domain";
import { gb } from "../domain";
import type { DiskInfo } from "../lib/api";
import { platformName } from "../lib/platform";
import CountUp from "./CountUp";
import Icon from "./Icon";

export default function Overview({
  disk,
  reclaimable,
  scanned,
  loading,
  due,
  onScan,
  onCancel,
  navigate,
}: {
  disk: DiskInfo | null;
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

  return (
    <>
      <div className="page-heading">
        <div>
          <p className="eyebrow">UM POUCO MAIS DE ESPAÇO. MUITO MAIS CLAREZA.</p>
          <h1>
            Seu computador,
            <br />
            <span>mais leve.</span>
          </h1>
          <p>Entenda o que ocupa espaço. Decida o que fica.</p>
        </div>
        <span className="device">
          <span className="status-dot" />
          Meu computador <small>{platformName()} · Protegido</small>
        </span>
      </div>
      {due && (
        <div className="callout amber-callout" role="status">
          <Icon name="clock" />
          <span>
            <strong>Sua limpeza de rotina está na hora.</strong> O lembrete que você configurou em Minha rotina pediu
            uma revisão.
          </span>
          <button type="button" className="button secondary" onClick={() => navigate("routine")}>
            Ver rotina
          </button>
        </div>
      )}
      <section className="overview-hero grain aurora" aria-label="Diagnóstico do disco">
        <div className="orbit-area">
          <div className="orbit">
            <div className="orbit-core">
              <span>DISCO {disk?.mount ? `· ${disk.mount}` : "LOCAL"}</span>
              <strong>{known ? <CountUp value={free} format={gb} /> : "—"}</strong>
              <small>{known ? `disponíveis de ${gb(total)}` : "lendo o disco…"}</small>
            </div>
          </div>
          <div className="orbit-caption">
            <span className="legend-dot yellow" />
            Espaço que você pode revisar
          </div>
        </div>
        <div className="hero-content">
          <span className="badge amber">
            <Icon name="spark" size={14} /> LIMPEZA COM CRITÉRIO
          </span>
          <h2>
            {loading
              ? "Olhando cada cantinho…"
              : scanned
                ? reclaimable === 0
                  ? "Tudo em ordem por aqui."
                  : "Tem espaço para respirar."
                : "Vamos encontrar o que sobra?"}
          </h2>
          <p>
            {loading
              ? "A análise de caches vasculha apenas pastas descartáveis com segurança."
              : scanned
                ? reclaimable === 0
                  ? "Nenhum arquivo desnecessário detectado neste momento."
                  : "Encontramos arquivos temporários e caches. Você revisa tudo antes de limpar."
                : "Uma análise rápida encontra caches e temporários. Seus arquivos pessoais continuam no lugar."}
          </p>
          <div className="reclaim-number">
            {loading ? "…" : scanned ? <CountUp value={reclaimable} format={gb} /> : "—"}
            <span>
              {loading
                ? "analisando caches do sistema"
                : scanned
                  ? "para revisar · tamanho estimado"
                  : "sua próxima análise começa aqui"}
            </span>
          </div>
          {loading ? (
            <div className="hero-actions">
              <button type="button" className="button secondary" onClick={onCancel}>
                Cancelar análise
              </button>
            </div>
          ) : (
            <div className="hero-actions">
              <button
                type="button"
                className={`button primary ${scanned && reclaimable > 0 ? "glow-button" : ""}`}
                onClick={scanned && reclaimable > 0 ? () => navigate("review") : onScan}
              >
                <Icon name={scanned && reclaimable > 0 ? "arrow" : "search"} />
                {scanned && reclaimable > 0 ? "Revisar limpeza" : "Analisar meu computador"}
              </button>
              {scanned && (
                <button type="button" className="text-button" onClick={onScan}>
                  <Icon name="refresh" size={16} />
                  Analisar novamente
                </button>
              )}
            </div>
          )}
          <div className="quiet-note">
            <Icon name="shield" size={16} />
            Você decide. Proteção total a arquivos de trabalho.
          </div>
        </div>
      </section>
      <div className="overview-bottom">
        <section className="panel disk-panel">
          <div className="section-heading">
            <h2>Um olhar sobre o disco</h2>
            <span className="muted">
              {disk?.mount ? `Unidade ${disk.mount} · ` : ""}
              {known ? `${gb(total)} total` : "—"}
            </span>
          </div>
          {known && (
            <div
              className="disk-bar flex h-4 w-full overflow-hidden rounded-full bg-[var(--color-line)]"
              role="img"
              aria-label={`Disco: ${gb(used)} ocupados (${usedPct}%), ${gb(free)} livres (${freePct}%)`}
            >
              <div className="bg-[#658fde] transition-all duration-300" style={{ width: `${usedPct}%` }} />
              <div className="bg-[#1d2430] transition-all duration-300" style={{ width: `${freePct}%` }} />
            </div>
          )}
          <div className="disk-legend">
            <span>
              <b className="legend-dot blue" />
              Espaço ocupado <strong>{known ? gb(used) : "—"}</strong>
            </span>
            <span>
              <b className="legend-dot dark" />
              Espaço livre <strong>{known ? gb(free) : "—"}</strong>
            </span>
            <span>
              <b className="legend-dot yellow" />
              Caches revisáveis <strong>{gb(reclaimable)}</strong>
            </span>
          </div>
        </section>
        <button type="button" className="panel safety-link" onClick={() => navigate("review")}>
          <span className="icon-tile green">
            <Icon name="shield" size={24} />
          </span>
          <div>
            <h2>Limpar sem perder o que importa</h2>
            <p>Veja o que protegemos e ajuste suas exclusões.</p>
            <span>
              Conhecer as proteções <Icon name="arrow" size={16} />
            </span>
          </div>
        </button>
      </div>
      <div className="principles stagger">
        <span>
          <Icon name="check" size={16} />
          Sem mexer em senhas ou histórico
        </span>
        <span>
          <Icon name="check" size={16} />
          Sem encerrar seus aplicativos
        </span>
        <span>
          <Icon name="check" size={16} />
          Cada decisão explicada
        </span>
      </div>
    </>
  );
}
