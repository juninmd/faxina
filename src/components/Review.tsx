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
    <div className="flex flex-col gap-6">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <p className="text-[11px] font-bold tracking-widest text-amber-400 uppercase">Você no controle</p>
          <h1 className="mt-1 text-3xl font-extrabold tracking-tight text-white sm:text-4xl">
            Revise antes de limpar.
          </h1>
          <p className="mt-1 text-sm text-slate-400">
            Caches de aplicativos e projetos nas unidades conectadas. Revise os locais antes de limpar.
          </p>
        </div>
        <div className="inline-flex items-center gap-2 rounded-full border border-emerald-500/20 bg-emerald-500/10 px-4 py-2 text-xs font-semibold text-emerald-400 shadow-xs">
          <Icon name="shield" size={16} />
          <span>Arquivos pessoais protegidos</span>
        </div>
      </div>

      <div className="flex flex-wrap items-center justify-between gap-4">
        <div className="flex items-center gap-2 rounded-2xl border border-slate-800/80 bg-slate-900/60 p-1.5 text-xs font-semibold backdrop-blur-md">
          <span className="flex items-center gap-1.5 rounded-xl bg-emerald-500/15 px-3 py-1.5 text-emerald-400">
            ✓ Analisar
          </span>
          <span className="h-0.5 w-4 bg-slate-700" />
          <span className="flex items-center gap-1.5 rounded-xl border border-amber-500/30 bg-amber-500/15 px-3 py-1.5 font-bold text-amber-300 shadow-xs">
            2. Revisar
          </span>
          <span className="h-0.5 w-4 bg-slate-700" />
          <span className="flex items-center gap-1.5 px-3 py-1.5 text-slate-400">3. Resultado</span>
        </div>

        {scanned && displayRules.length > 0 && (
          <button
            type="button"
            className="inline-flex cursor-pointer items-center gap-2 rounded-xl border border-slate-700/80 bg-slate-850 px-4 py-2 text-xs font-semibold text-slate-200 transition-all hover:border-slate-600 hover:bg-slate-800 disabled:opacity-50"
            disabled={loading}
            onClick={onScan}
          >
            <Icon name="refresh" size={15} />
            <span>{loading ? "Analisando unidades…" : "Analisar novamente todas as unidades"}</span>
          </button>
        )}
      </div>

      {!scanned || (!displayRules.length && !loading) ? (
        <section className="flex flex-col items-center justify-center gap-4 rounded-3xl border border-slate-800/80 bg-slate-900/60 p-12 text-center shadow-lg backdrop-blur-md">
          <span className="flex h-16 w-16 items-center justify-center rounded-2xl border border-slate-700 bg-slate-800/80 text-amber-400">
            <Icon name="search" size={32} />
          </span>
          <h2 className="text-xl font-bold text-white">
            {loading
              ? "Analisando caches nas unidades…"
              : !scanned
                ? "Primeiro, uma análise."
                : "Nenhum cache encontrado."}
          </h2>
          <p className="max-w-md text-sm text-slate-400">
            {loading
              ? "Buscando caches de aplicativos e projetos em todas as unidades acessíveis. Isso pode levar alguns minutos."
              : !scanned
                ? "Execute a análise para listar os itens que podem ser limpos com segurança."
                : "Nenhum item das regras de limpeza foi encontrado nos locais acessíveis."}
          </p>
          <button
            type="button"
            className="mt-2 inline-flex cursor-pointer items-center gap-2 rounded-xl bg-gradient-to-r from-amber-500 to-amber-400 px-6 py-3 text-sm font-bold text-slate-950 shadow-lg shadow-amber-500/20 hover:brightness-110 disabled:opacity-50"
            disabled={loading}
            onClick={onScan}
          >
            <Icon name="refresh" size={16} />
            <span>{loading ? "Analisando…" : "Analisar novamente"}</span>
          </button>
        </section>
      ) : (
        <>
          <div className="grid grid-cols-1 items-start gap-6 lg:grid-cols-12">
            <section className="flex flex-col gap-3 rounded-2xl border border-slate-800/80 bg-slate-900/60 p-5 shadow-lg backdrop-blur-md lg:col-span-7">
              <div className="flex items-center justify-between pb-2 border-b border-slate-800/60">
                <h2 className="text-base font-bold text-white">Itens encontrados</h2>
                <span className="text-xs font-medium text-slate-400">{displayRules.length} categorias</span>
              </div>
              <div className="flex flex-col gap-2">
                {displayRules.map((r) => {
                  const ignored = excluded.includes(r.id);
                  const isSelected = picked.some((p) => p.id === r.id);
                  const isDetail = detail.id === r.id;

                  return (
                    <div
                      key={r.id}
                      className={`group flex items-center gap-3 rounded-xl border p-3 transition-all ${
                        isDetail
                          ? "border-amber-500/40 bg-amber-500/5 shadow-xs"
                          : isSelected
                            ? "border-slate-700/80 bg-slate-850/60"
                            : "border-slate-800/70 bg-slate-900/40 hover:border-slate-700 hover:bg-slate-850/40"
                      } ${r.blocked || ignored ? "opacity-60" : ""}`}
                    >
                      <input
                        type="checkbox"
                        aria-label={`Selecionar ${r.app}: ${r.name}`}
                        checked={isSelected}
                        disabled={r.blocked || ignored}
                        onChange={() => onSelect(r.id)}
                        className="h-4 w-4 cursor-pointer rounded border-slate-700 accent-amber-500 disabled:cursor-not-allowed"
                      />
                      <button
                        type="button"
                        className="flex min-w-0 flex-1 cursor-pointer items-center gap-3 text-left"
                        onClick={() => onDetail(r)}
                      >
                        <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-gradient-to-br from-slate-700 to-slate-800 font-bold text-sm text-white shadow-xs">
                          {r.app.slice(0, 1)}
                        </span>
                        <span className="flex min-w-0 flex-1 flex-col">
                          <strong className="truncate text-sm font-semibold text-white group-hover:text-amber-300">
                            {r.name}
                          </strong>
                          <span className="text-xs text-slate-400">
                            {r.app} · {count(r.files)} arquivos
                          </span>
                          {(r.blocked || ignored || r.busy || r.caution) && (
                            <span
                              className={`mt-0.5 text-[11px] font-medium ${
                                r.blocked || ignored ? "text-slate-400" : "text-amber-400"
                              }`}
                            >
                              {r.blocked
                                ? "Protegido · dados pessoais"
                                : ignored
                                  ? "Excluído por você"
                                  : r.busy
                                    ? "App aberto · será ignorado se bloqueado"
                                    : r.caution}
                            </span>
                          )}
                        </span>
                        <b className="font-mono text-sm font-bold text-slate-200">{gb(r.size)}</b>
                        <span className="text-slate-400 transition-transform group-hover:translate-x-0.5 group-hover:text-white">
                          ›
                        </span>
                      </button>
                    </div>
                  );
                })}
              </div>
              <div className="mt-2 flex items-center gap-2 pt-3 border-t border-slate-800/60 text-xs text-slate-400">
                <Icon name="shield" size={15} />
                <span>Dados pessoais, favoritos e senhas nunca são tocados.</span>
              </div>
            </section>

            <aside className="sticky top-20 flex flex-col gap-4 rounded-2xl border border-slate-800/80 bg-slate-900/80 p-6 shadow-xl backdrop-blur-md lg:col-span-5">
              <span className="text-[10px] font-bold tracking-widest text-amber-400 uppercase">Por dentro do item</span>
              <div className="flex items-center gap-3">
                <span className="flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl border border-slate-700 bg-slate-800 text-amber-400 shadow-sm">
                  <Icon name={detail.blocked ? "shield" : "box"} size={24} />
                </span>
                <div className="flex flex-col min-w-0">
                  <h2 className="truncate text-lg font-bold text-white">{detail.name}</h2>
                  <span className="text-xs text-slate-400">
                    {detail.app} · {detail.group}
                  </span>
                </div>
              </div>

              <div className="h-px bg-slate-800" />

              <div>
                <h3 className="text-xs font-bold text-slate-300 uppercase tracking-wider">O que acontece ao limpar?</h3>
                <p className="mt-1 text-sm leading-relaxed text-slate-300">{detail.consequence}</p>
              </div>

              <div>
                <h3 className="text-xs font-bold text-slate-300 uppercase tracking-wider">Local no disco</h3>
                <div className="mt-1 flex flex-col gap-1.5">
                  {(detail.paths?.length ? detail.paths : [detail.path]).map((path) => (
                    <code
                      key={path}
                      className="block rounded-xl border border-slate-800 bg-slate-950/70 p-2 font-mono text-xs text-slate-300 break-all select-all"
                    >
                      {path}
                    </code>
                  ))}
                </div>
              </div>

              <div
                className={`flex items-start gap-3 rounded-xl border p-3.5 text-xs leading-relaxed ${
                  detail.blocked
                    ? "border-amber-500/30 bg-amber-500/10 text-amber-200"
                    : "border-emerald-500/30 bg-emerald-500/10 text-emerald-200"
                }`}
              >
                <Icon name="shield" size={16} />
                <span>
                  {detail.blocked
                    ? "Este item está protegido por conter dados importantes e não pode ser incluído."
                    : "Esta regra mira apenas arquivos descartáveis. Confira o impacto antes de selecionar."}
                </span>
              </div>

              {!detail.blocked && (
                <button
                  type="button"
                  className="mt-1 w-full cursor-pointer rounded-xl border border-slate-700 bg-slate-800/80 px-4 py-2.5 text-xs font-semibold text-slate-200 transition-colors hover:bg-slate-750"
                  onClick={() => onExclude(detail.id)}
                >
                  {excluded.includes(detail.id) ? "Voltar a incluir nas análises" : "Nunca limpar este item"}
                </button>
              )}
            </aside>
          </div>

          <div className="sticky bottom-4 z-20 flex flex-wrap items-center justify-between gap-4 rounded-2xl border border-slate-700/80 bg-slate-950/95 p-4 shadow-2xl backdrop-blur-xl">
            <div className="flex items-center gap-4">
              <span className="font-mono text-2xl font-extrabold text-amber-400">{gb(totalSize(picked))}</span>
              <span className="text-xs text-slate-400 font-medium">
                {picked.length} {picked.length === 1 ? "categoria selecionada" : "categorias selecionadas"}
              </span>
            </div>
            <div className="flex items-center gap-2 rounded-full border border-slate-800 bg-slate-900 px-3 py-1 text-xs text-slate-300">
              <Icon name="trash" size={14} />
              <span>Destino: Lixeira / Exclusão</span>
            </div>
            <button
              type="button"
              className="inline-flex cursor-pointer items-center gap-2 rounded-xl bg-gradient-to-r from-amber-500 to-amber-400 px-6 py-3 text-sm font-bold text-slate-950 shadow-lg shadow-amber-500/20 transition-all hover:brightness-110 active:scale-[0.98] disabled:cursor-not-allowed disabled:opacity-50"
              disabled={!picked.length || loading}
              onClick={onConfirm}
            >
              <span>Continuar</span>
              <Icon name="arrow" size={16} />
            </button>
          </div>
        </>
      )}
    </div>
  );
}
