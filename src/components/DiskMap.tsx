import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import type { useScan } from "../hooks/useScan";
import { api, type DiskInfo } from "../lib/api";
import { formatBytes } from "../lib/format";
import type { Mark } from "../lib/marks";
import { Collector } from "./Collector";
import Icon from "./Icon";
import { MapView } from "./MapView";
import ScanProgress from "./ScanProgress";

export default function DiskMap({
  active,
  scan,
  home,
  marked,
  onToggleMark,
  onClearMarked,
  onUnmark,
  onDeleteMarked,
}: {
  active: boolean;
  scan: ReturnType<typeof useScan>;
  home: string;
  marked: Map<string, Mark>;
  onToggleMark: (m: Mark) => void;
  onClearMarked: () => void;
  onUnmark: (path: string) => void;
  onDeleteMarked: (permanent: boolean) => void;
}) {
  const [disks, setDisks] = useState<DiskInfo[]>([]);
  const [expanded, setExpanded] = useState(false);
  const [notice, setNotice] = useState("");

  useEffect(() => {
    api.listDisks().then(setDisks, () => setDisks([]));
  }, []);

  useEffect(() => {
    if (!active || !expanded) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !document.querySelector("dialog[open]")) setExpanded(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [active, expanded]);

  if (!active) return null;
  // A failed scan or folder open is stored in the hook; show it where the picker's errors go.
  const message = notice || scan.error;

  const chooseFolder = async () => {
    try {
      const dir = await open({ directory: true });
      if (typeof dir === "string") {
        onClearMarked();
        setNotice("");
        scan.scan(dir);
      }
    } catch (e) {
      setNotice(`Não foi possível abrir o seletor: ${e}`);
    }
  };

  const markedList = [...marked.values()];

  return (
    <section className={`disk-map-screen ${expanded ? "is-expanded" : ""}`} aria-label="Explorador visual do disco">
      <div className="page-heading compact">
        <div>
          <p className="eyebrow">O MAPA QUE VOCÊ JÁ CONHECE</p>
          <h1>O disco inteiro, à vista.</h1>
          <p>Blocos, anéis e cada pasta no seu lugar. Explore antes de decidir.</p>
        </div>
        <button type="button" className="button secondary" onClick={() => setExpanded((value) => !value)}>
          <Icon name="grid" size={17} />
          {expanded ? "Sair da tela cheia" : "Expandir mapa"}
        </button>
      </div>

      <div className="map-drive-toolbar">
        {disks.length > 0 && (
          <label>
            Disco
            <select
              aria-label="Escolher disco do mapa"
              value={scan.root}
              disabled={scan.progress !== null}
              onChange={(e) => {
                onClearMarked();
                setNotice("");
                scan.scan(e.target.value);
              }}
            >
              {!disks.some((d) => d.mount === scan.root) && scan.root && <option value={scan.root}>{scan.root}</option>}
              {disks.map((disk) => (
                <option key={disk.mount} value={disk.mount}>
                  Unidade {disk.mount} ({formatBytes(disk.free)} livres de {formatBytes(disk.total)})
                </option>
              ))}
            </select>
          </label>
        )}
        <div className="map-toolbar-actions">
          <button type="button" className="button secondary" disabled={scan.progress !== null} onClick={chooseFolder}>
            Escolher outra pasta…
          </button>
          {scan.view && (
            <button
              type="button"
              className="button secondary"
              disabled={scan.progress !== null}
              onClick={() => scan.scan(scan.root)}
            >
              <Icon name="refresh" size={16} />
              Reanalisar
            </button>
          )}
        </div>
      </div>

      {message && (
        <div className="map-notice" role="alert">
          {message}
        </div>
      )}

      {scan.progress ? (
        <ScanProgress progress={scan.progress} path={scan.scanning} onCancel={api.cancelScan} />
      ) : scan.view ? (
        <div className="native-map panel native-map-panel flex flex-col">
          <MapView scan={scan} marked={marked} onToggleMark={onToggleMark} />
          <Collector items={markedList} onClear={onClearMarked} onUnmark={onUnmark} onDelete={onDeleteMarked} />
        </div>
      ) : (
        <div className="panel map-scan-state flex flex-col items-center justify-center gap-8 p-12 text-center">
          <div>
            <div className="text-6xl" aria-hidden="true">
              💽
            </div>
            <h2 className="mt-4 text-2xl font-semibold">Selecione uma unidade ou pasta para mapear</h2>
            <p className="mx-auto mt-2 max-w-md text-sm text-[var(--color-muted)]">
              O mapa visual analisa o tamanho real de cada pasta e arquivo em blocos e anéis interativos.
            </p>
          </div>
          <div className="flex flex-wrap items-center justify-center gap-3">
            {home && (
              <button type="button" className="button primary" onClick={() => scan.scan(home)}>
                Mapear pasta pessoal
              </button>
            )}
            {disks.map((d) => (
              <button key={d.mount} type="button" className="button secondary" onClick={() => scan.scan(d.mount)}>
                Mapear disco {d.mount}
              </button>
            ))}
            <button type="button" className="button secondary" onClick={chooseFolder}>
              Escolher pasta…
            </button>
          </div>
        </div>
      )}
    </section>
  );
}
