import { getVersion } from "@tauri-apps/api/app";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { useEffect, useState } from "react";
import Icon from "./Icon";

export default function Updates() {
  const [state, setState] = useState<"idle" | "checking" | "ready" | "downloading" | "uptodate" | "error">("idle");
  const [update, setUpdate] = useState<Update | null>(null);
  const [pct, setPct] = useState(0);
  const [errorMsg, setErrorMsg] = useState("");
  const [version, setVersion] = useState("");

  useEffect(() => {
    getVersion().then(setVersion, () => setVersion(""));
  }, []);

  const checkForUpdates = async () => {
    setState("checking");
    setErrorMsg("");
    try {
      const found = await check({ timeout: 15000 });
      if (found?.available) {
        setUpdate(found);
        setState("ready");
      } else {
        setState("uptodate");
      }
    } catch (e) {
      setErrorMsg(String(e));
      setState("error");
    }
  };

  const install = async () => {
    if (!update) return;
    setState("downloading");
    setPct(0);
    let total = 0;
    let got = 0;
    try {
      await update.downloadAndInstall((ev) => {
        if (ev.event === "Started") total = ev.data.contentLength ?? 0;
        if (ev.event === "Progress") {
          got += ev.data.chunkLength;
          setPct(total ? Math.round((got / total) * 100) : 0);
        }
      });
      await relaunch();
    } catch (e) {
      setErrorMsg(String(e));
      setState("error");
    }
  };

  return (
    <>
      <div className="page-heading compact">
        <div>
          <p className="eyebrow">CONFIANÇA DO DOWNLOAD À ATUALIZAÇÃO</p>
          <h1>Uma origem. Tudo verificável.</h1>
          <p>Atualizações transparentes, com a sua escolha de quando instalar.</p>
        </div>
        <span className="badge">Canal estável</span>
      </div>
      <section className="panel update-hero">
        <span className="update-art">
          <Icon name="download" size={54} />
        </span>
        <div>
          <span className="eyebrow">FAXINA · {version ? `VERSÃO ATUAL: v${version}` : "VERIFICAÇÃO DE VERSÃO"}</span>
          <h2>
            {state === "checking"
              ? "Verificando se há atualizações…"
              : state === "ready"
                ? `Nova versão ${update?.version} disponível!`
                : state === "downloading"
                  ? `Baixando atualização (${pct}%)…`
                  : state === "uptodate"
                    ? "Você está usando a versão mais recente."
                    : state === "error"
                      ? "Não foi possível verificar atualizações."
                      : "Seu sistema limpo, rápido e seguro."}
          </h2>
          <p>
            {state === "ready"
              ? update?.body || "Melhorias de desempenho, regras de limpeza atualizadas e correções de estabilidade."
              : state === "uptodate"
                ? "Nenhuma atualização pendente no momento. As assinaturas criptográficas garantem a integridade dos pacotes."
                : state === "error"
                  ? `Erro: ${errorMsg}. Verifique sua conexão com a internet.`
                  : "Pacotes verificados com chave pública Minisign. Nenhuma alteração é instalada sem sua autorização."}
          </p>
          <div className="flex items-center gap-3">
            {state === "ready" ? (
              <button type="button" className="button primary" onClick={install}>
                <Icon name="download" size={18} />
                Baixar e instalar agora
              </button>
            ) : (
              <button
                type="button"
                className="button primary"
                disabled={state === "checking" || state === "downloading"}
                onClick={checkForUpdates}
              >
                <Icon name="refresh" size={18} />
                {state === "checking"
                  ? "Verificando…"
                  : state === "downloading"
                    ? "Baixando…"
                    : "Verificar atualizações"}
              </button>
            )}
          </div>
          {state === "ready" && (
            <p className="quiet-note" role="status">
              Ao concluir o download, o aplicativo será reiniciado automaticamente.
            </p>
          )}
        </div>
      </section>
      <section className="panel delivery-panel">
        <div className="section-heading">
          <h2>Critérios de distribuição confiável</h2>
          <span className="muted">Segurança e transparência</span>
        </div>
        {[
          [
            "shield",
            "Integridade da atualização",
            "Assinatura criptográfica verificada por Minisign antes de qualquer extração.",
            "Ativo no Faxina",
          ],
          [
            "file",
            "Identidade do publicador",
            "Executáveis empacotados pelo pipeline oficial com hashes SHA-256.",
            "Auditável via GitHub",
          ],
          [
            "refresh",
            "Atualização sem perder preferências",
            "Suas exclusões, rotinas e histórico de limpeza são mantidos entre versões.",
            "Garantido",
          ],
          [
            "box",
            "Instalação e recuperação",
            "Instalação limpa via NSIS no Windows e bundles assinados no macOS e Linux.",
            "Multiplataforma",
          ],
        ].map(([icon, title, description, status]) => (
          <div className="delivery-row" key={title}>
            <span className="icon-tile">
              <Icon name={icon as "shield" | "file" | "refresh" | "box"} />
            </span>
            <div>
              <strong>{title}</strong>
              <p>{description}</p>
            </div>
            <span className="badge green">{status}</span>
          </div>
        ))}
      </section>
      <div className="callout green-callout">
        <Icon name="shield" />
        <span>
          O Faxina é de código aberto. Toda release é gerada e assinada publicamente nos workflows do repositório
          oficial.
        </span>
      </div>
    </>
  );
}
