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
    <div className="flex flex-col gap-6">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <p className="text-[11px] font-bold tracking-widest text-amber-400 uppercase">
            Cobertura com responsabilidade
          </p>
          <h1 className="mt-1 text-3xl font-extrabold tracking-tight text-white sm:text-4xl">
            Cada aplicativo, uma regra.
          </h1>
          <p className="mt-1 text-sm text-slate-400">O que pode sair, o que deve ficar e por quê.</p>
        </div>
        <span className="rounded-full border border-slate-700/80 bg-slate-800/60 px-4 py-2 text-xs font-semibold text-slate-300 shadow-xs backdrop-blur-sm">
          Catálogo de regras
        </span>
      </div>

      <div className="flex items-center gap-3 rounded-2xl border border-emerald-500/30 bg-emerald-500/10 p-4 text-xs leading-relaxed text-emerald-200">
        <Icon name="shield" size={18} />
        <span>
          Uma regra só entra no Faxina após validação de remoção e preservação. Seus dados pessoais, senhas e trabalho
          são sempre preservados.
        </span>
      </div>

      <div className="flex flex-wrap items-center gap-3">
        <div className="relative flex flex-1 min-w-64 items-center">
          <span className="absolute left-3.5 text-slate-400">
            <Icon name="search" size={16} />
          </span>
          <input
            aria-label="Buscar aplicativo"
            placeholder="Buscar aplicativo ou categoria…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            className="w-full rounded-xl border border-slate-700/80 bg-slate-900/80 py-2.5 pr-4 pl-10 text-xs text-slate-200 outline-none transition-colors focus:border-amber-500"
          />
        </div>

        <label className="sr-only" htmlFor="group-filter">
          Filtrar por grupo
        </label>
        <select
          id="group-filter"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          className="cursor-pointer rounded-xl border border-slate-700/80 bg-slate-900/80 px-4 py-2.5 text-xs font-medium text-slate-200 outline-none transition-colors focus:border-amber-500"
        >
          {["Todos", "Navegadores", "Sistema", "Desenvolvimento", "Aplicativos"].map((v) => (
            <option key={v}>{v}</option>
          ))}
        </select>
      </div>

      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {filtered.map((r) => (
          <article
            key={r.id}
            className="flex flex-col justify-between gap-4 rounded-2xl border border-slate-800/80 bg-slate-900/60 p-5 shadow-lg backdrop-blur-md transition-all hover:border-slate-700 hover:bg-slate-850/50"
          >
            <div>
              <div className="flex items-center justify-between gap-2">
                <span className="flex h-9 w-9 items-center justify-center rounded-xl bg-gradient-to-br from-slate-700 to-slate-800 font-bold text-sm text-white shadow-xs">
                  {r.app[0]}
                </span>
                <span
                  className={`rounded-full px-2.5 py-0.5 text-[10px] font-bold uppercase tracking-wider ${
                    r.blocked
                      ? "border border-amber-500/30 bg-amber-500/10 text-amber-400"
                      : "border border-emerald-500/30 bg-emerald-500/10 text-emerald-400"
                  }`}
                >
                  {r.blocked ? "Bloqueado por segurança" : "Regra ativa"}
                </span>
              </div>
              <h2 className="mt-3 text-base font-bold text-white">{r.app}</h2>
              <p className="text-xs font-medium text-slate-400">{r.name}</p>
              <p className="mt-2 text-xs leading-relaxed text-slate-300">{r.consequence}</p>
            </div>

            <div className="flex flex-col gap-3 pt-3 border-t border-slate-800/60">
              <div className="flex items-center gap-2 text-[11px] text-slate-400">
                <Icon name={r.blocked ? "shield" : "check"} size={14} />
                <span>{r.blocked ? "Dados offline preservados" : "Escopo e impacto documentados"}</span>
              </div>
              <button
                type="button"
                className="w-full cursor-pointer rounded-xl border border-slate-700 bg-slate-800/80 py-2 text-xs font-semibold text-slate-200 transition-colors hover:bg-slate-750 disabled:cursor-not-allowed disabled:opacity-50"
                disabled={r.blocked}
                onClick={() => onExclude(r.id)}
              >
                {r.blocked
                  ? "Sempre protegido"
                  : excluded.includes(r.id)
                    ? "Remover exclusão"
                    : "Excluir da minha limpeza"}
              </button>
            </div>
          </article>
        ))}
      </div>

      {!filtered.length && (
        <section className="flex flex-col items-center justify-center gap-4 rounded-3xl border border-slate-800/80 bg-slate-900/60 p-12 text-center shadow-lg backdrop-blur-md">
          <span className="flex h-14 w-14 items-center justify-center rounded-2xl border border-slate-700 bg-slate-800 text-amber-400">
            <Icon name="search" size={28} />
          </span>
          <h2 className="text-lg font-bold text-white">Nenhum aplicativo encontrado.</h2>
          <p className="text-sm text-slate-400">Tente outro nome ou grupo.</p>
          <button
            type="button"
            className="cursor-pointer rounded-xl border border-slate-700 bg-slate-800 px-5 py-2 text-xs font-semibold text-slate-200 transition-colors hover:bg-slate-750"
            onClick={() => {
              setQuery("");
              setFilter("Todos");
            }}
          >
            Limpar filtros
          </button>
        </section>
      )}
    </div>
  );
}
