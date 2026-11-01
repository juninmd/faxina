import { useEffect, useRef, useState } from "react";
import { count, gb, type Page, type Report, shareSummary } from "../domain";
import { formatBytes } from "../lib/format";
import CountUp from "./CountUp";
import Icon from "./Icon";

export default function ReportPanel({
  report,
  history,
  navigate,
}: {
  report: Report | null;
  history: Report[];
  navigate: (p: Page) => void;
}) {
  const [copied, setCopied] = useState<"ok" | "error" | null>(null);
  const timer = useRef<number | undefined>(undefined);
  const summary = report ? shareSummary(report, formatBytes) : undefined;
  useEffect(() => () => window.clearTimeout(timer.current), []);
  const share = async () => {
    if (!summary) return;
    try {
      await navigator.clipboard.writeText(summary);
      setCopied("ok");
    } catch {
      setCopied("error");
    }
    // One timer at a time, so a quick second click is not reset by the first.
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => setCopied(null), 2000);
  };

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <p className="text-[11px] font-bold tracking-widest text-amber-400 uppercase">Resultado sem letras miúdas</p>
          <h1 className="mt-1 text-3xl font-extrabold tracking-tight text-white sm:text-4xl">Você sabe o que mudou.</h1>
          <p className="mt-1 text-sm text-slate-400">Arquivos removidos e espaço disponível são medidas diferentes.</p>
        </div>
        <div className="inline-flex items-center gap-2 rounded-full border border-slate-700/80 bg-slate-800/60 px-4 py-2 text-xs font-semibold text-slate-300 shadow-xs backdrop-blur-sm">
          <Icon name="check" size={14} />
          <span>Resultado verificado</span>
        </div>
      </div>

      {report ? (
        <>
          <section className="flex flex-col items-start gap-6 rounded-3xl border border-slate-800/80 bg-gradient-to-br from-slate-900/90 via-[#0e1626] to-slate-950 p-6 shadow-xl backdrop-blur-xl sm:flex-row sm:items-center sm:p-8">
            <span className="flex h-16 w-16 shrink-0 items-center justify-center rounded-2xl border border-emerald-500/30 bg-emerald-500/15 text-emerald-400 shadow-md">
              <Icon name="check" size={32} />
            </span>
            <div className="flex flex-col">
              <span className="text-[10px] font-bold tracking-widest text-emerald-400 uppercase">
                Limpeza concluída
              </span>
              <h2 className="mt-1 text-2xl font-bold tracking-tight text-white">
                {report.destination === "trash"
                  ? "Tudo no lugar. Inclusive na Lixeira."
                  : "Limpeza permanente concluída."}
              </h2>
              <p className="mt-1 text-sm text-slate-400">
                {count(report.files)} arquivos processados.{" "}
                {report.skipped ? `${report.skipped} item(ns) mantido(s) por segurança.` : "Nenhum item foi mantido."}
              </p>
            </div>
          </section>

          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
            <div className="flex flex-col rounded-2xl border border-slate-800/80 bg-slate-900/60 p-5 shadow-lg backdrop-blur-md">
              <span className="text-xs font-medium text-slate-400">Selecionado</span>
              <strong className="mt-2 font-mono text-3xl font-extrabold text-white">
                <CountUp value={report.selected} format={gb} />
              </strong>
              <small className="mt-1 text-[11px] text-slate-400">Tamanho aparente estimado</small>
            </div>

            <div className="flex flex-col rounded-2xl border border-slate-800/80 bg-slate-900/60 p-5 shadow-lg backdrop-blur-md">
              <span className="text-xs font-medium text-slate-400">Enviado à Lixeira</span>
              <strong className="mt-2 font-mono text-3xl font-extrabold text-white">
                <CountUp value={report.trashed} format={gb} />
              </strong>
              <small className="mt-1 text-[11px] text-slate-400">Ainda ocupa espaço no disco</small>
            </div>

            <div className="flex flex-col rounded-2xl border border-slate-800/80 bg-slate-900/60 p-5 shadow-lg backdrop-blur-md">
              <span className="text-xs font-medium text-slate-400">Excluído definitivamente</span>
              <strong className="mt-2 font-mono text-3xl font-extrabold text-white">
                <CountUp value={report.permanent} format={gb} />
              </strong>
              <small className="mt-1 text-[11px] text-slate-400">Sem recuperação pela Lixeira</small>
            </div>

            <div className="flex flex-col rounded-2xl border border-amber-500/40 bg-gradient-to-br from-amber-500/10 via-slate-900/80 to-slate-950 p-5 shadow-xl backdrop-blur-md">
              <span className="text-xs font-bold text-amber-300 uppercase tracking-wider">
                Espaço liberado no disco
              </span>
              <strong className="mt-2 font-mono text-3xl font-extrabold text-amber-400">
                +<CountUp value={report.observed} format={gb} />
              </strong>
              <small className="mt-1 text-[11px] text-amber-300/80">Espaço liberado imediatamente</small>
            </div>
          </div>

          <div className="flex items-center gap-3 rounded-2xl border border-amber-500/30 bg-amber-500/10 p-4 text-xs leading-relaxed text-amber-200">
            <Icon name="file" size={18} />
            <span>
              {report.destination === "trash"
                ? "Enviar à Lixeira não libera espaço imediatamente. Você pode restaurar os arquivos pelo sistema caso precise."
                : "A exclusão permanente removeu os arquivos diretamente do disco, liberando espaço de armazenamento."}
            </span>
          </div>

          <div className="flex flex-wrap items-center gap-3">
            <button
              type="button"
              className="inline-flex cursor-pointer items-center gap-2 rounded-xl bg-gradient-to-r from-amber-500 to-amber-400 px-5 py-2.5 text-xs font-bold text-slate-950 shadow-lg shadow-amber-500/20 hover:brightness-110 active:scale-[0.98]"
              onClick={() => navigate("overview")}
            >
              <span>Voltar ao diagnóstico</span>
              <Icon name="arrow" size={15} />
            </button>
            {summary && (
              <button
                type="button"
                className="inline-flex cursor-pointer items-center gap-2 rounded-xl border border-slate-700 bg-slate-800/80 px-4 py-2.5 text-xs font-semibold text-slate-200 transition-colors hover:bg-slate-750"
                onClick={share}
              >
                <Icon name="copy" size={15} />
                <span>
                  {copied === "ok"
                    ? "Resumo copiado!"
                    : copied === "error"
                      ? "Não foi possível copiar"
                      : "Copiar resumo"}
                </span>
              </button>
            )}
          </div>
        </>
      ) : (
        <section className="flex flex-col items-center justify-center gap-4 rounded-3xl border border-slate-800/80 bg-slate-900/60 p-12 text-center shadow-lg backdrop-blur-md">
          <span className="flex h-16 w-16 items-center justify-center rounded-2xl border border-slate-700 bg-slate-800 text-amber-400">
            <Icon name="file" size={32} />
          </span>
          <h2 className="text-xl font-bold text-white">Seu primeiro resultado aparece aqui.</h2>
          <p className="max-w-md text-sm text-slate-400">
            Faça uma análise e execute uma limpeza para ver os números separados com transparência.
          </p>
          <button
            type="button"
            className="mt-2 inline-flex cursor-pointer items-center gap-2 rounded-xl bg-gradient-to-r from-amber-500 to-amber-400 px-6 py-3 text-sm font-bold text-slate-950 shadow-lg shadow-amber-500/20 hover:brightness-110"
            onClick={() => navigate("overview")}
          >
            <span>Ir para diagnóstico</span>
            <Icon name="arrow" size={16} />
          </button>
        </section>
      )}

      <section className="flex flex-col gap-4 rounded-2xl border border-slate-800/80 bg-slate-900/60 p-6 shadow-lg backdrop-blur-md">
        <div className="flex items-center justify-between pb-3 border-b border-slate-800/60">
          <h2 className="text-base font-bold text-white">Histórico local</h2>
          <span className="text-xs font-medium text-slate-400">Salvo neste computador</span>
        </div>
        {history.length ? (
          // biome-ignore lint/a11y/noNoninteractiveTabindex: horizontal overflow must be keyboard-scrollable on narrow screens
          <section className="overflow-x-auto" aria-label="Histórico de limpezas" tabIndex={0}>
            <table className="w-full text-left text-xs">
              <thead>
                <tr className="border-b border-slate-800/80 text-[11px] font-bold text-slate-400 uppercase tracking-wider">
                  <th className="pb-3 pl-2">Quando</th>
                  <th className="pb-3">Destino</th>
                  <th className="pb-3">Processado</th>
                  <th className="pb-3 pr-2">Liberado</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-800/40">
                {history.map((r) => (
                  <tr key={r.id} className="transition-colors hover:bg-slate-800/30">
                    <td className="py-3 pl-2 text-slate-300 font-medium">{new Date(r.date).toLocaleString("pt-BR")}</td>
                    <td className="py-3">
                      <span
                        className={`inline-flex rounded-full px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide ${
                          r.destination === "trash"
                            ? "border border-emerald-500/30 bg-emerald-500/10 text-emerald-400"
                            : "border border-rose-500/30 bg-rose-500/10 text-rose-400"
                        }`}
                      >
                        {r.destination === "trash" ? "Lixeira" : "Permanente"}
                      </span>
                    </td>
                    <td className="py-3 font-mono text-slate-300">{gb(r.removed)}</td>
                    <td className="py-3 pr-2 font-mono font-bold text-amber-400">+{gb(r.observed)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </section>
        ) : (
          <p className="text-xs text-slate-400">Nenhuma limpeza registrada ainda.</p>
        )}
      </section>
    </div>
  );
}
