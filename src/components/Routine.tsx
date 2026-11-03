import { nextCleanup, type Report, type Rule, type Settings } from "../domain";
import Icon from "./Icon";

const DATE = new Intl.DateTimeFormat("pt-BR", { day: "2-digit", month: "long", hour: "2-digit", minute: "2-digit" });

export default function Routine({
  settings,
  rules,
  history,
  update,
  storageError,
}: {
  settings: Settings;
  rules: Rule[];
  history: Report[];
  update: (s: Settings) => void;
  storageError: boolean;
}) {
  const next = nextCleanup(settings, history);
  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <p className="text-[11px] font-bold tracking-widest text-amber-400 uppercase">Menos uma coisa para lembrar</p>
          <h1 className="mt-1 text-3xl font-extrabold tracking-tight text-white sm:text-4xl">
            Uma rotina do seu jeito.
          </h1>
          <p className="mt-1 text-sm text-slate-400">Suas preferências ficam salvas. O controle continua com você.</p>
        </div>
        <div
          className={`inline-flex items-center gap-2 rounded-full border px-4 py-2 text-xs font-semibold shadow-xs backdrop-blur-sm ${
            storageError
              ? "border-amber-500/30 bg-amber-500/10 text-amber-300"
              : "border-emerald-500/20 bg-emerald-500/10 text-emerald-400"
          }`}
        >
          <Icon name={storageError ? "file" : "check"} size={15} />
          <span>{storageError ? "Não foi possível salvar" : "Preferências locais"}</span>
        </div>
      </div>

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-12">
        <section className="flex flex-col gap-5 rounded-2xl border border-slate-800/80 bg-slate-900/60 p-6 shadow-lg backdrop-blur-md lg:col-span-7">
          <div className="flex items-center justify-between pb-3 border-b border-slate-800/60">
            <h2 className="text-base font-bold text-white">Planejar uma limpeza</h2>
            <span className="text-amber-400">
              <Icon name="clock" size={20} />
            </span>
          </div>

          <div className="flex items-center justify-between gap-4 rounded-xl border border-slate-800/70 bg-slate-950/40 p-4">
            <div className="flex flex-col">
              <strong className="text-sm font-semibold text-white">Lembrete de limpeza</strong>
              <p className="text-xs text-slate-400 mt-0.5">
                Avisar quando o intervalo escolhido passar desde a sua última limpeza registrada.
              </p>
            </div>
            <label className="relative inline-flex cursor-pointer items-center">
              <input
                type="checkbox"
                role="switch"
                aria-checked={settings.schedule}
                aria-label="Ativar lembrete de limpeza"
                checked={settings.schedule}
                onChange={(e) => update({ ...settings, schedule: e.target.checked })}
                className="peer sr-only"
              />
              <div className="peer h-6 w-11 rounded-full bg-slate-700 peer-checked:bg-amber-500 peer-focus-visible:ring-2 peer-focus-visible:ring-amber-500 peer-focus-visible:ring-offset-2 peer-focus-visible:ring-offset-slate-900 after:absolute after:top-[2px] after:left-[2px] after:h-5 after:w-5 after:rounded-full after:bg-white after:transition-all after:content-[''] peer-checked:after:translate-x-full peer-checked:after:border-white" />
            </label>
          </div>

          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <label className="flex flex-col gap-1.5 text-xs font-semibold text-slate-300">
              <span>Frequência</span>
              <select
                id="routine-frequency"
                aria-label="Frequência do lembrete"
                disabled={!settings.schedule}
                value={settings.frequency}
                onChange={(e) => update({ ...settings, frequency: e.target.value })}
                className="rounded-xl border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-slate-200 outline-none focus:border-amber-500 disabled:opacity-50"
              >
                <option>Semanal</option>
                <option>Mensal</option>
              </select>
            </label>
            <label className="flex flex-col gap-1.5 text-xs font-semibold text-slate-300">
              <span>Horário</span>
              <input
                id="routine-time"
                aria-label="Horário do lembrete"
                type="time"
                disabled={!settings.schedule}
                value={settings.time}
                onChange={(e) => {
                  if (e.target.value) update({ ...settings, time: e.target.value });
                }}
                className="rounded-xl border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-slate-200 outline-none focus:border-amber-500 disabled:opacity-50"
              />
            </label>
          </div>

          <div className="flex items-center justify-between gap-4 rounded-xl border border-slate-800/70 bg-slate-950/40 p-4">
            <div className="flex flex-col">
              <strong className="text-sm font-semibold text-white">Avisar na tela inicial</strong>
              <p className="text-xs text-slate-400 mt-0.5">
                Mostrar o lembrete na visão geral quando a rotina estiver na hora de revisar.
              </p>
            </div>
            <label className="relative inline-flex cursor-pointer items-center">
              <input
                type="checkbox"
                role="switch"
                aria-checked={settings.reminder}
                aria-label="Avisar na tela inicial"
                checked={settings.reminder}
                onChange={(e) => update({ ...settings, reminder: e.target.checked })}
                className="peer sr-only"
              />
              <div className="peer h-6 w-11 rounded-full bg-slate-700 peer-checked:bg-amber-500 peer-focus-visible:ring-2 peer-focus-visible:ring-amber-500 peer-focus-visible:ring-offset-2 peer-focus-visible:ring-offset-slate-900 after:absolute after:top-[2px] after:left-[2px] after:h-5 after:w-5 after:rounded-full after:bg-white after:transition-all after:content-[''] peer-checked:after:translate-x-full peer-checked:after:border-white" />
            </label>
          </div>

          <div className="flex items-center gap-3 rounded-xl border border-amber-500/30 bg-amber-500/10 p-4 text-xs leading-relaxed text-amber-200">
            <Icon name="clock" size={18} />
            <span>
              {!settings.schedule
                ? "Lembrete desativado. "
                : !settings.reminder
                  ? "Aviso na tela inicial desativado — o prazo continua calculado aqui. "
                  : next
                    ? `Próximo lembrete: ${DATE.format(next)}. `
                    : "O primeiro lembrete será agendado após a sua próxima limpeza. "}
              O Faxina não roda em segundo plano: o aviso aparece quando você abre o aplicativo.
            </span>
          </div>
        </section>

        <aside className="flex flex-col gap-4 rounded-2xl border border-emerald-500/20 bg-gradient-to-br from-emerald-950/20 via-slate-900/60 to-slate-900/60 p-6 shadow-xl backdrop-blur-md lg:col-span-5">
          <span className="flex h-12 w-12 items-center justify-center rounded-2xl border border-emerald-500/30 bg-emerald-500/15 text-emerald-400 shadow-sm">
            <Icon name="shield" size={24} />
          </span>
          <div className="flex flex-col">
            <h2 className="text-lg font-bold text-white">Autonomia com limites.</h2>
            <p className="mt-1 text-sm leading-relaxed text-slate-400">
              A limpeza respeita rigorosamente suas exclusões e mantém arquivos em uso. Exclusão definitiva sempre exige
              sua confirmação explícita.
            </p>
          </div>

          <ul className="mt-2 flex flex-col gap-2.5 text-xs text-slate-300">
            <li className="flex items-center gap-2">
              <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-emerald-500/20 text-emerald-400">
                <Icon name="check" size={12} />
              </span>
              <span>Sem encerrar aplicativos à força</span>
            </li>
            <li className="flex items-center gap-2">
              <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-emerald-500/20 text-emerald-400">
                <Icon name="check" size={12} />
              </span>
              <span>Sem mexer em dados pessoais ou senhas</span>
            </li>
            <li className="flex items-center gap-2">
              <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-emerald-500/20 text-emerald-400">
                <Icon name="check" size={12} />
              </span>
              <span>Com relatório detalhado a cada execução</span>
            </li>
          </ul>
        </aside>
      </div>

      <section className="flex flex-col gap-4 rounded-2xl border border-slate-800/80 bg-slate-900/60 p-6 shadow-lg backdrop-blur-md">
        <div className="flex items-center justify-between pb-3 border-b border-slate-800/60">
          <div>
            <h2 className="text-base font-bold text-white">Suas exclusões</h2>
            <p className="text-xs text-slate-400">Estes itens ficam fora da seleção recomendada.</p>
          </div>
          <span className="rounded-full border border-slate-700 bg-slate-800 px-3 py-1 text-xs font-semibold text-slate-300">
            {settings.excluded.length} {settings.excluded.length === 1 ? "item" : "itens"}
          </span>
        </div>

        {settings.excluded.length ? (
          <div className="flex flex-col gap-2">
            {settings.excluded.map((id) => {
              const r = rules.find((rule) => rule.id === id);
              return (
                <div
                  key={id}
                  className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-slate-800/70 bg-slate-950/40 p-3 transition-colors hover:border-slate-700"
                >
                  <span className="flex flex-col min-w-0">
                    <strong className="text-sm font-semibold text-white">{r ? `${r.app} · ${r.name}` : id}</strong>
                    {r && <code className="mt-0.5 font-mono text-xs text-slate-400 break-all">{r.path}</code>}
                  </span>
                  <button
                    type="button"
                    className="cursor-pointer text-xs font-semibold text-rose-400 hover:text-rose-300"
                    onClick={() => update({ ...settings, excluded: settings.excluded.filter((x) => x !== id) })}
                  >
                    Remover exclusão
                  </button>
                </div>
              );
            })}
          </div>
        ) : (
          <p className="text-xs text-slate-400">
            Você ainda não excluiu nenhum item. Adicione exclusões em Revisar limpeza ou Aplicativos.
          </p>
        )}
      </section>
    </div>
  );
}
