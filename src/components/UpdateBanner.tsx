import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { useEffect, useState } from "react";

type State =
  | { kind: "idle" }
  | { kind: "available"; update: Update }
  | { kind: "downloading"; pct: number }
  | { kind: "error"; msg: string };

/** Checks GitHub Releases once at startup; installing is always the user's call. */
export function UpdateBanner() {
  const [state, setState] = useState<State>({ kind: "idle" });

  useEffect(() => {
    if (import.meta.env.DEV) return;
    check({ timeout: 15000 })
      .then((update) => update && setState({ kind: "available", update }))
      .catch((e) => console.warn("update check failed", e));
  }, []);

  if (state.kind === "idle") return null;

  const install = async (update: Update) => {
    let total = 0;
    let got = 0;
    setState({ kind: "downloading", pct: 0 });
    try {
      await update.downloadAndInstall((ev) => {
        if (ev.event === "Started") total = ev.data.contentLength ?? 0;
        if (ev.event === "Progress") {
          got += ev.data.chunkLength;
          setState({ kind: "downloading", pct: total ? Math.round((got / total) * 100) : 0 });
        }
      });
      await relaunch();
    } catch (e) {
      setState({ kind: "error", msg: String(e) });
    }
  };

  return (
    <div
      className="pop-in flex items-center justify-center gap-3 bg-[var(--color-accent)] px-4 py-1.5 text-sm text-black"
      role="status"
    >
      {state.kind === "available" && (
        <>
          <span>
            ✨ Nova versão <b>{state.update.version}</b> disponível
          </span>
          <button
            type="button"
            onClick={() => install(state.update)}
            className="rounded bg-black/85 px-3 py-0.5 text-[var(--color-accent)]"
          >
            Atualizar e reiniciar
          </button>
          <button type="button" onClick={() => setState({ kind: "idle" })} className="underline">
            Depois
          </button>
        </>
      )}
      {state.kind === "downloading" && <span>Baixando atualização… {state.pct}%</span>}
      {state.kind === "error" && <span>Falha ao atualizar: {state.msg}</span>}
    </div>
  );
}
