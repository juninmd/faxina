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
    <div className="flex flex-col gap-6">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <p className="text-[11px] font-bold tracking-widest text-amber-400 uppercase">
            Confiança do download à atualização
          </p>
          <h1 className="mt-1 text-3xl font-extrabold tracking-tight text-white sm:text-4xl">
            Uma origem. Tudo verificável.
          </h1>
          <p className="mt-1 text-sm text-slate-400">
            Atualizações transparentes, com a sua escolha de quando instalar.
          </p>
        </div>
        <span className="rounded-full border border-emerald-500/20 bg-emerald-500/10 px-4 py-2 text-xs font-semibold text-emerald-400 shadow-xs backdrop-blur-sm">
          Canal estável
        </span>
      </div>

      <section className="flex flex-col items-start gap-6 rounded-3xl border border-slate-800/80 bg-gradient-to-br from-slate-900/90 via-[#0e1626] to-slate-950 p-6 shadow-xl backdrop-blur-xl sm:p-8 md:flex-row md:items-center">
        <span className="flex h-20 w-20 shrink-0 items-center justify-center rounded-3xl border border-slate-700 bg-slate-800/80 text-amber-400 shadow-md">
          <Icon name="download" size={40} />
        </span>
        <div className="flex flex-1 flex-col">
          <span className="text-[10px] font-bold tracking-widest text-amber-400 uppercase">
            FAXINA · {version ? `VERSÃO ATUAL: v${version}` : "VERIFICAÇÃO DE VERSÃO"}
          </span>
          <h2 className="mt-1 text-2xl font-bold tracking-tight text-white sm:text-3xl">
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
          <p className="mt-2 text-sm leading-relaxed text-slate-300">
            {state === "ready"
              ? update?.body || "Melhorias de desempenho, regras de limpeza atualizadas e correções de estabilidade."
              : state === "uptodate"
                ? "Nenhuma atualização pendente no momento. As assinaturas criptográficas garantem a integridade dos pacotes."
                : state === "error"
                  ? `Erro: ${errorMsg}. Verifique sua conexão com a internet.`
                  : "Pacotes verificados com chave pública Minisign. Nenhuma alteração é instalada sem sua autorização."}
          </p>

          {state === "downloading" && (
            <div className="mt-4 flex w-full flex-col gap-1.5">
              <div className="h-3 w-full overflow-hidden rounded-full border border-slate-700 bg-slate-800">
                <div
                  className="h-full rounded-full bg-gradient-to-r from-amber-500 to-amber-400 transition-all duration-300"
                  style={{ width: `${pct}%` }}
                />
              </div>
              <span className="text-right font-mono text-xs text-amber-400">{pct}%</span>
            </div>
          )}

          <div className="mt-6 flex flex-wrap items-center gap-3">
            {state === "ready" ? (
              <button
                type="button"
                className="inline-flex cursor-pointer items-center gap-2 rounded-xl bg-gradient-to-r from-amber-500 to-amber-400 px-6 py-3 text-sm font-bold text-slate-950 shadow-lg shadow-amber-500/20 hover:brightness-110 active:scale-[0.98]"
                onClick={install}
              >
                <Icon name="download" size={16} />
                <span>Baixar e instalar agora</span>
              </button>
            ) : (
              <button
                type="button"
                className="inline-flex cursor-pointer items-center gap-2 rounded-xl bg-gradient-to-r from-amber-500 to-amber-400 px-6 py-3 text-sm font-bold text-slate-950 shadow-lg shadow-amber-500/20 hover:brightness-110 active:scale-[0.98] disabled:cursor-not-allowed disabled:opacity-50"
                disabled={state === "checking" || state === "downloading"}
                onClick={checkForUpdates}
              >
                <Icon name="refresh" size={16} />
                <span>
                  {state === "checking"
                    ? "Verificando…"
                    : state === "downloading"
                      ? "Baixando…"
                      : "Verificar atualizações"}
                </span>
              </button>
            )}
          </div>
          {state === "ready" && (
            <p className="mt-3 text-xs text-slate-400" role="status">
              Ao concluir o download, o aplicativo será reiniciado automaticamente.
            </p>
          )}
        </div>
      </section>

      <section className="flex flex-col gap-4 rounded-2xl border border-slate-800/80 bg-slate-900/60 p-6 shadow-lg backdrop-blur-md">
        <div className="flex items-center justify-between pb-3 border-b border-slate-800/60">
          <h2 className="text-base font-bold text-white">Critérios de distribuição confiável</h2>
          <span className="text-xs font-medium text-slate-400">Segurança e transparência</span>
        </div>

        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
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
              "Atualização sem perder dados",
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
            <div
              key={title}
              className="flex items-start gap-3.5 rounded-xl border border-slate-800/70 bg-slate-950/40 p-4 transition-colors hover:border-slate-700"
            >
              <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-slate-700 bg-slate-800 text-amber-400 shadow-xs">
                <Icon name={icon as "shield" | "file" | "refresh" | "box"} size={18} />
              </span>
              <div className="flex flex-1 flex-col">
                <div className="flex items-center justify-between gap-2">
                  <strong className="text-sm font-semibold text-white">{title}</strong>
                  <span className="rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 text-[10px] font-bold text-emerald-400 uppercase tracking-wide">
                    {status}
                  </span>
                </div>
                <p className="mt-1 text-xs leading-relaxed text-slate-400">{description}</p>
              </div>
            </div>
          ))}
        </div>
      </section>

      <div className="flex items-center gap-3 rounded-2xl border border-emerald-500/30 bg-emerald-500/10 p-4 text-xs leading-relaxed text-emerald-200">
        <Icon name="shield" size={18} />
        <span>
          O Faxina é de código aberto. Toda release é gerada e assinada publicamente nos workflows do repositório
          oficial.
        </span>
      </div>
    </div>
  );
}
