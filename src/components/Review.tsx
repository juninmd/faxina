import { count, gb, type Rule, totalSize } from "../domain";
import Icon from "./Icon";

export default function Review({
  rules,
  scanned,
  selected,
  excluded,
  detail,
  onSelect,
  onDetail,
  onExclude,
  onConfirm,
  onScan,
  loading = false,
}: {
  rules: Rule[];
  scanned: boolean;
  selected: string[];
  excluded: string[];
  detail: Rule;
  onSelect: (id: string) => void;
  onDetail: (rule: Rule) => void;
  onExclude: (id: string) => void;
  onConfirm: () => void;
  onScan: () => void;
  loading?: boolean;
}) {
  const displayRules = rules.filter((r) => r.size > 0 || r.blocked);
  const picked = displayRules.filter((r) => selected.includes(r.id) && !excluded.includes(r.id) && !r.blocked);

  return (
    <>
      <div className="page-heading compact">
        <div>
          <p className="eyebrow">VOCÊ NO CONTROLE</p>
          <h1>Revise antes de limpar.</h1>
          <p>Cada item tem um motivo. Nenhuma surpresa depois.</p>
        </div>
        <span className="badge green">
          <Icon name="shield" size={15} />
          Arquivos pessoais protegidos
        </span>
      </div>
      <div className="stepper">
        <span className="done">✓ Analisar</span>
        <i />
        <span className="active">
          2 <b>Revisar</b>
        </span>
        <i />
        <span>3 Resultado</span>
      </div>
      {!scanned || (!displayRules.length && !loading) ? (
        <section className="empty-state panel">
          <Icon name="search" size={38} />
          <h2>
            {loading
              ? "Analisando caches do sistema…"
              : !scanned
                ? "Primeiro, uma análise."
                : "Nenhum cache encontrado."}
          </h2>
          <p>
            {loading
              ? "Buscando caches, arquivos temporários e logs que podem ser limpos com segurança."
              : !scanned
                ? "Execute a análise para listar os itens que podem ser limpos com segurança."
                : "Seu computador já está limpo! Nenhum arquivo desnecessário foi detectado."}
          </p>
          <button type="button" className="button primary" disabled={loading} onClick={onScan}>
            <Icon name="refresh" size={16} />
            {loading ? "Analisando…" : "Analisar novamente"}
          </button>
        </section>
      ) : (
        <>
          <div className="review-grid">
            <section className="panel rule-list">
              <div className="section-heading">
                <h2>Itens encontrados</h2>
                <span className="muted">{displayRules.length} categorias</span>
              </div>
              {displayRules.map((r) => {
                const ignored = excluded.includes(r.id);
                return (
                  <div
                    className={`rule-row ${detail.id === r.id ? "is-detail" : ""} ${r.blocked || ignored ? "is-muted" : ""}`}
                    key={r.id}
                  >
                    <input
                      type="checkbox"
                      aria-label={`Selecionar ${r.app}: ${r.name}`}
                      checked={picked.some((p) => p.id === r.id)}
                      disabled={r.blocked || ignored}
                      onChange={() => onSelect(r.id)}
                    />
                    <button type="button" className="rule-detail-button" onClick={() => onDetail(r)}>
                      <span className={`app-monogram app-${r.id}`}>{r.app.slice(0, 1)}</span>
                      <span className="rule-name">
                        <strong>{r.name}</strong>
                        <small>
                          {r.app} · {count(r.files)} arquivos
                        </small>
                        {(r.blocked || ignored || r.busy || r.caution) && (
                          <em className={r.blocked || ignored ? "muted" : "warning-text"}>
                            {r.blocked
                              ? "Protegido · dados pessoais"
                              : ignored
                                ? "Excluído por você"
                                : r.busy
                                  ? "App aberto · será ignorado se bloqueado"
                                  : r.caution}
                          </em>
                        )}
                      </span>
                      <b className="rule-size">{gb(r.size)}</b>
                      <span className="detail-arrow">›</span>
                    </button>
                  </div>
                );
              })}
              <div className="list-footer">
                <Icon name="shield" size={16} />
                Dados pessoais, favoritos e senhas nunca são tocados.
              </div>
            </section>
            <aside className="panel detail-panel">
              <span className="eyebrow">POR DENTRO DO ITEM</span>
              <div className="detail-symbol">
                <Icon name={detail.blocked ? "shield" : "box"} size={28} />
              </div>
              <h2>{detail.name}</h2>
              <span className="muted">
                {detail.app} · {detail.group}
              </span>
              <div className="detail-divider" />
              <h3>O que acontece ao limpar?</h3>
              <p>{detail.consequence}</p>
              <h3>Local no disco</h3>
              <code>{detail.path}</code>
              <div className={`callout ${detail.blocked ? "amber-callout" : "green-callout"}`}>
                <Icon name="shield" size={18} />
                <span>
                  {detail.blocked
                    ? "Este item está protegido por conter dados importantes e não pode ser incluído."
                    : "Esta regra mira apenas arquivos descartáveis. Confira o impacto antes de selecionar."}
                </span>
              </div>
              {!detail.blocked && (
                <button type="button" className="button secondary full" onClick={() => onExclude(detail.id)}>
                  {excluded.includes(detail.id) ? "Voltar a incluir nas análises" : "Nunca limpar este item"}
                </button>
              )}
            </aside>
          </div>
          <div className="selection-bar">
            <div>
              <strong>{gb(totalSize(picked))}</strong>
              <span>{picked.length} categorias selecionadas</span>
            </div>
            <span className="selection-destination">
              <Icon name="trash" size={18} />
              Destino: Lixeira / Exclusão
            </span>
            <button type="button" className="button primary" disabled={!picked.length} onClick={onConfirm}>
              Continuar <Icon name="arrow" size={18} />
            </button>
          </div>
        </>
      )}
    </>
  );
}
