import { useState } from "react";
import type { Rule } from "../domain";
import Icon from "./Icon";

export default function Coverage({
  rules,
  excluded,
  onExclude,
}: {
  rules: Rule[];
  excluded: string[];
  onExclude: (id: string) => void;
}) {
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState("Todos");
  const filtered = rules.filter(
    (r) =>
      (filter === "Todos" || r.group === filter) &&
      `${r.app} ${r.name}`.toLocaleLowerCase("pt-BR").includes(query.toLocaleLowerCase("pt-BR")),
  );

  return (
    <>
      <div className="page-heading compact">
        <div>
          <p className="eyebrow">COBERTURA COM RESPONSABILIDADE</p>
          <h1>Cada aplicativo, uma regra.</h1>
          <p>O que pode sair, o que deve ficar e por quê.</p>
        </div>
        <span className="badge">Catálogo de regras</span>
      </div>
      <div className="callout green-callout">
        <Icon name="shield" />
        <span>
          Uma regra só entra no Faxina após validação de remoção e preservação. Seus dados pessoais, senhas e trabalho
          são sempre preservados.
        </span>
      </div>
      <div className="filter-toolbar">
        <label className="search-field">
          <Icon name="search" size={18} />
          <input
            aria-label="Buscar aplicativo"
            placeholder="Buscar aplicativo ou categoria…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </label>
        <label className="sr-only" htmlFor="group-filter">
          Filtrar por grupo
        </label>
        <select id="group-filter" value={filter} onChange={(e) => setFilter(e.target.value)}>
          {["Todos", "Navegadores", "Sistema", "Desenvolvimento", "Aplicativos"].map((v) => (
            <option key={v}>{v}</option>
          ))}
        </select>
      </div>
      <div className="coverage-grid">
        {filtered.map((r) => (
          <article key={r.id} className="panel coverage-card">
            <div className="section-heading">
              <span className={`app-monogram app-${r.id}`}>{r.app[0]}</span>
              <span className={`badge ${r.blocked ? "amber" : "green"}`}>
                {r.blocked ? "Bloqueado por segurança" : "Regra ativa"}
              </span>
            </div>
            <h2>{r.app}</h2>
            <p className="muted">{r.name}</p>
            <p>{r.consequence}</p>
            <div className="coverage-proof">
              <Icon name={r.blocked ? "shield" : "check"} size={16} />
              {r.blocked ? "Dados offline preservados" : "Escopo e impacto documentados"}
            </div>
            <button
              type="button"
              className="button secondary full"
              disabled={r.blocked}
              onClick={() => onExclude(r.id)}
            >
              {r.blocked
                ? "Sempre protegido"
                : excluded.includes(r.id)
                  ? "Remover exclusão"
                  : "Excluir da minha limpeza"}
            </button>
          </article>
        ))}
      </div>
      {!filtered.length && (
        <section className="panel empty-state">
          <Icon name="search" size={32} />
          <h2>Nenhum aplicativo encontrado.</h2>
          <p>Tente outro nome ou grupo.</p>
          <button
            type="button"
            className="button secondary"
            onClick={() => {
              setQuery("");
              setFilter("Todos");
            }}
          >
            Limpar filtros
          </button>
        </section>
      )}
    </>
  );
}
