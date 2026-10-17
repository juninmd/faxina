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
    <>
      <div className="page-heading compact">
        <div>
          <p className="eyebrow">MENOS UMA COISA PARA LEMBRAR</p>
          <h1>Uma rotina do seu jeito.</h1>
          <p>Suas preferências ficam salvas. O controle continua com você.</p>
        </div>
        <span className={`badge ${storageError ? "amber" : "green"}`}>
          <Icon name={storageError ? "file" : "check"} size={15} />
          {storageError ? "Não foi possível salvar" : "Preferências locais"}
        </span>
      </div>
      <div className="routine-grid">
        <section className="panel settings-panel">
          <div className="section-heading">
            <h2>Planejar uma limpeza</h2>
            <Icon name="clock" />
          </div>
          <div className="setting-row">
            <div>
              <strong>Lembrete de limpeza</strong>
              <p>Avisar quando o intervalo escolhido passar desde a sua última limpeza registrada.</p>
            </div>
            <input
              type="checkbox"
              role="switch"
              aria-checked={settings.schedule}
              aria-label="Ativar lembrete de limpeza"
              checked={settings.schedule}
              onChange={(e) => update({ ...settings, schedule: e.target.checked })}
            />
          </div>
          <div className="form-grid">
            <label>
              Frequência
              <select
                disabled={!settings.schedule}
                value={settings.frequency}
                onChange={(e) => update({ ...settings, frequency: e.target.value })}
              >
                <option>Semanal</option>
                <option>Mensal</option>
              </select>
            </label>
            <label>
              Horário
              <input
                type="time"
                disabled={!settings.schedule}
                value={settings.time}
                onChange={(e) => {
                  if (e.target.value) update({ ...settings, time: e.target.value });
                }}
              />
            </label>
          </div>
          <div className="setting-row">
            <div>
              <strong>Avisar na tela inicial</strong>
              <p>Mostrar o lembrete na visão geral quando a rotina estiver na hora de revisar.</p>
            </div>
            <input
              type="checkbox"
              role="switch"
              aria-checked={settings.reminder}
              aria-label="Avisar na tela inicial"
              checked={settings.reminder}
              onChange={(e) => update({ ...settings, reminder: e.target.checked })}
            />
          </div>
          <div className="callout amber-callout">
            <Icon name="clock" />
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
        <aside className="panel routine-aside">
          <span className="icon-tile green">
            <Icon name="shield" size={28} />
          </span>
          <h2>Autonomia com limites.</h2>
          <p>
            A limpeza respeita rigorosamente suas exclusões e mantém arquivos em uso. Exclusão definitiva sempre exige
            sua confirmação explícita.
          </p>
          <ul className="check-list">
            <li>
              <Icon name="check" size={16} />
              Sem encerrar aplicativos à força
            </li>
            <li>
              <Icon name="check" size={16} />
              Sem mexer em dados pessoais ou senhas
            </li>
            <li>
              <Icon name="check" size={16} />
              Com relatório detalhado a cada execução
            </li>
          </ul>
        </aside>
      </div>
      <section className="panel exclusion-panel">
        <div className="section-heading">
          <div>
            <h2>Suas exclusões</h2>
            <p className="muted">Estes itens ficam fora da seleção recomendada.</p>
          </div>
          <span className="badge">{settings.excluded.length} itens</span>
        </div>
        {settings.excluded.length ? (
          settings.excluded.map((id) => {
            const r = rules.find((rule) => rule.id === id);
            return (
              <div key={id} className="exclusion-row">
                <span>
                  <strong>{r ? `${r.app} · ${r.name}` : id}</strong>
                  {r && <code>{r.path}</code>}
                </span>
                <button
                  type="button"
                  className="text-button"
                  onClick={() => update({ ...settings, excluded: settings.excluded.filter((x) => x !== id) })}
                >
                  Remover exclusão
                </button>
              </div>
            );
          })
        ) : (
          <p className="muted">
            Você ainda não excluiu nenhum item. Adicione exclusões em Revisar limpeza ou Aplicativos.
          </p>
        )}
      </section>
    </>
  );
}
