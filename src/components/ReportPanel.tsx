import { useState } from "react";
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
  const [copied, setCopied] = useState(false);
  const summary = report ? shareSummary(report, formatBytes) : undefined;
  const share = async () => {
    if (!summary) return;
    try {
      await navigator.clipboard.writeText(summary);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      setCopied(false);
    }
  };

  return (
    <>
      <div className="page-heading compact">
        <div>
          <p className="eyebrow">RESULTADO SEM LETRAS MIÚDAS</p>
          <h1>Você sabe o que mudou.</h1>
          <p>Arquivos removidos e espaço disponível são medidas diferentes.</p>
        </div>
        <span className="badge">Resultado verificado</span>
      </div>
      {report ? (
        <>
          <section className="panel report-hero">
            <span className="success-circle">
              <Icon name="check" size={32} />
            </span>
            <div>
              <span className="eyebrow">LIMPEZA CONCLUÍDA</span>
              <h2>
                {report.destination === "trash"
                  ? "Tudo no lugar. Inclusive na Lixeira."
                  : "Limpeza permanente concluída."}
              </h2>
              <p>
                {count(report.files)} arquivos processados.{" "}
                {report.skipped ? `${report.skipped} item(ns) mantido(s) por segurança.` : "Nenhum item foi mantido."}
              </p>
            </div>
          </section>
          <div className="metric-grid stagger">
            <div className="panel metric">
              <span>Selecionado</span>
              <strong>
                <CountUp value={report.selected} format={gb} />
              </strong>
              <small>Tamanho aparente estimado</small>
            </div>
            <div className="panel metric">
              <span>Enviado à Lixeira</span>
              <strong>
                <CountUp value={report.trashed} format={gb} />
              </strong>
              <small>Ainda ocupa espaço no disco</small>
            </div>
            <div className="panel metric">
              <span>Excluído definitivamente</span>
              <strong>
                <CountUp value={report.permanent} format={gb} />
              </strong>
              <small>Sem possibilidade de restaurar pela Lixeira</small>
            </div>
            <div className="panel metric emphasized">
              <span>Espaço liberado no disco</span>
              <strong>
                +<CountUp value={report.observed} format={gb} />
              </strong>
              <small>Espaço liberado imediatamente</small>
            </div>
          </div>
          <div className="callout amber-callout">
            <Icon name="file" />
            <span>
              {report.destination === "trash"
                ? "Enviar à Lixeira não libera espaço imediatamente. Você pode restaurar os arquivos pelo sistema caso precise."
                : "A exclusão permanente removeu os arquivos diretamente do disco, liberando espaço de armazenamento."}
            </span>
          </div>
          <div className="flex items-center gap-3">
            <button type="button" className="button secondary" onClick={() => navigate("overview")}>
              Voltar ao diagnóstico <Icon name="arrow" size={18} />
            </button>
            {summary && (
              <button type="button" className="button secondary" onClick={share}>
                <Icon name="copy" size={18} />
                {copied ? "Resumo copiado!" : "Copiar resumo"}
              </button>
            )}
          </div>
        </>
      ) : (
        <section className="panel empty-state">
          <Icon name="file" size={38} />
          <h2>Seu primeiro resultado aparece aqui.</h2>
          <p>Faça uma análise e execute uma limpeza para ver os números separados.</p>
          <button type="button" className="button primary" onClick={() => navigate("overview")}>
            Ir para diagnóstico
          </button>
        </section>
      )}
      <section className="panel history-panel">
        <div className="section-heading">
          <h2>Histórico local</h2>
          <span className="muted">Salvo neste computador</span>
        </div>
        {history.length ? (
          // biome-ignore lint/a11y/noNoninteractiveTabindex: horizontal overflow must be keyboard-scrollable on narrow screens
          <section className="table-scroll" aria-label="Histórico de limpezas" tabIndex={0}>
            <table>
              <thead>
                <tr>
                  <th>Quando</th>
                  <th>Destino</th>
                  <th>Processado</th>
                  <th>Liberado</th>
                </tr>
              </thead>
              <tbody>
                {history.map((r) => (
                  <tr key={r.id}>
                    <td>{new Date(r.date).toLocaleString("pt-BR")}</td>
                    <td>{r.destination === "trash" ? "Lixeira" : "Permanente"}</td>
                    <td>{gb(r.removed)}</td>
                    <td>+{gb(r.observed)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </section>
        ) : (
          <p className="muted">Nenhuma limpeza registrada ainda.</p>
        )}
      </section>
    </>
  );
}
