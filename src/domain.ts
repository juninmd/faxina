import type { JunkItem } from "./lib/api";
import { formatBytes, formatCount } from "./lib/format";

export type Page = "overview" | "map" | "bigfiles" | "review" | "report" | "dupes" | "coverage" | "routine" | "updates";

export type Rule = {
  id: string;
  name: string;
  app: string;
  group: string;
  size: number;
  files: number;
  path: string;
  consequence: string;
  caution?: string;
  blocked?: boolean;
  busy?: boolean;
};

export const BASE_RULES: Rule[] = [
  {
    id: "temp",
    name: "Temporários antigos",
    app: "Windows",
    group: "Sistema",
    size: 0,
    files: 0,
    path: "AppData/Local/Temp",
    consequence: "Somente arquivos com mais de 24 horas. Itens em uso serão mantidos.",
  },
  {
    id: "inetcache",
    name: "Cache da Internet do Windows",
    app: "Windows",
    group: "Sistema",
    size: 0,
    files: 0,
    path: "AppData/Local/Microsoft/Windows/INetCache",
    consequence: "Arquivos temporários e páginas em cache do sistema.",
  },
  {
    id: "wincaches",
    name: "Caches do shell e do Windows",
    app: "Windows",
    group: "Sistema",
    size: 0,
    files: 0,
    path: "AppData/Local/Microsoft/Windows/Caches",
    consequence: "Caches de miniaturas e áreas de trabalho remotas. Recriados quando necessário.",
  },
  {
    id: "crashdumps",
    name: "Despejos de falhas",
    app: "Windows",
    group: "Sistema",
    size: 0,
    files: 0,
    path: "AppData/Local/CrashDumps",
    consequence: "Relatórios de falhas de aplicativos encerrados com erro.",
  },
  {
    id: "wer",
    name: "Relatórios de erro",
    app: "Windows",
    group: "Sistema",
    size: 0,
    files: 0,
    path: "AppData/Local/Microsoft/Windows/WER",
    consequence: "Filas e arquivos de relatórios de erros arquivados do sistema.",
  },
  {
    id: "shaders",
    name: "Cache de shaders",
    app: "DirectX / GPU",
    group: "Sistema",
    size: 0,
    files: 0,
    path: "AppData/Local/D3DSCache",
    consequence: "Será recriado pelo sistema. Jogos podem levar mais tempo na primeira abertura.",
    caution: "Pode afetar a primeira abertura",
  },
  {
    id: "uwp",
    name: "Apps da Microsoft Store",
    app: "Microsoft Store",
    group: "Sistema",
    size: 0,
    files: 0,
    path: "AppData/Local/Packages/*/AC",
    consequence: "Caches e arquivos temporários de aplicativos da Store.",
  },
  {
    id: "chrome",
    name: "Cache de páginas",
    app: "Google Chrome",
    group: "Navegadores",
    size: 0,
    files: 0,
    path: "AppData/Local/Google/Chrome/User Data/Default/Cache",
    consequence: "Imagens e páginas serão baixadas novamente. Seus acessos e favoritos ficam preservados.",
  },
  {
    id: "edge",
    name: "Cache do Microsoft Edge",
    app: "Microsoft Edge",
    group: "Navegadores",
    size: 0,
    files: 0,
    path: "AppData/Local/Microsoft/Edge/User Data/Default/Cache",
    consequence: "Imagens e páginas serão baixadas novamente. Favoritos e senhas preservados.",
  },
  {
    id: "brave",
    name: "Cache do Brave",
    app: "Brave",
    group: "Navegadores",
    size: 0,
    files: 0,
    path: "AppData/Local/BraveSoftware/Brave-Browser/User Data/Default/Cache",
    consequence: "Caches e arquivos temporários do navegador Brave.",
  },
  {
    id: "vivaldi",
    name: "Cache do Vivaldi",
    app: "Vivaldi",
    group: "Navegadores",
    size: 0,
    files: 0,
    path: "AppData/Local/Vivaldi/User Data/Default/Cache",
    consequence: "Caches do navegador Vivaldi.",
  },
  {
    id: "firefox",
    name: "Cache do Firefox",
    app: "Firefox",
    group: "Navegadores",
    size: 0,
    files: 0,
    path: "AppData/Local/Mozilla/Firefox/Profiles/*/*cache*",
    consequence: "Caches de páginas e miniaturas do Firefox. Histórico e favoritos preservados.",
  },
  {
    id: "thunderbird",
    name: "Cache do correio",
    app: "Thunderbird",
    group: "Navegadores",
    size: 0,
    files: 0,
    path: "AppData/Local/Thunderbird/Profiles/*/*cache*",
    consequence: "Caches locais do Thunderbird. Suas mensagens e contas ficam preservadas.",
  },
  {
    id: "vscode",
    name: "Cache e logs",
    app: "VS Code",
    group: "Aplicativos",
    size: 0,
    files: 0,
    path: "AppData/Roaming/Code/Cache",
    consequence: "Caches de renderização e logs. Suas extensões e configurações não são afetadas.",
  },
  {
    id: "discord",
    name: "Cache de imagens",
    app: "Discord",
    group: "Aplicativos",
    size: 0,
    files: 0,
    path: "AppData/Roaming/discord/Cache",
    consequence: "Feche o Discord antes de tentar novamente se estiver aberto. Nenhum processo será encerrado.",
    busy: true,
  },
  {
    id: "slack",
    name: "Cache do Slack",
    app: "Slack",
    group: "Aplicativos",
    size: 0,
    files: 0,
    path: "AppData/Roaming/Slack/Cache",
    consequence: "Caches e logs do Slack. O aplicativo baixa novamente as mensagens que precisar.",
  },
  {
    id: "spotify",
    name: "Cache de músicas e capas",
    app: "Spotify",
    group: "Aplicativos",
    size: 0,
    files: 0,
    path: "AppData/Local/Spotify/Data",
    consequence: "Músicas e capas em cache. Músicas offline podem precisar ser baixadas de novo.",
  },
  {
    id: "npm",
    name: "Cache de pacotes",
    app: "npm",
    group: "Desenvolvimento",
    size: 0,
    files: 0,
    path: "AppData/Local/npm-cache",
    consequence:
      "O próximo download de dependências pode demorar mais. Projetos e arquivos de código ficam preservados.",
  },
  {
    id: "yarn",
    name: "Cache do Yarn",
    app: "Yarn",
    group: "Desenvolvimento",
    size: 0,
    files: 0,
    path: "AppData/Local/Yarn/Cache",
    consequence: "Pacotes baixados pelo Yarn. Serão baixados novamente sob demanda.",
  },
  {
    id: "pip",
    name: "Cache do pip",
    app: "pip",
    group: "Desenvolvimento",
    size: 0,
    files: 0,
    path: "AppData/Local/pip/cache",
    consequence: "Pacotes Python baixados pelo pip.",
  },
  {
    id: "cargo",
    name: "Cache de crates",
    app: "Cargo",
    group: "Desenvolvimento",
    size: 0,
    files: 0,
    path: ".cargo/registry/cache",
    consequence: "Arquivos .crate baixados. O Cargo baixa de novo se compilar do zero.",
  },
  {
    id: "gradle",
    name: "Caches do Gradle",
    app: "Gradle",
    group: "Desenvolvimento",
    size: 0,
    files: 0,
    path: ".gradle/caches",
    consequence: "Wrappers e artefatos em cache do Gradle.",
  },
  {
    id: "rustup",
    name: "Downloads do Rustup",
    app: "Rustup",
    group: "Desenvolvimento",
    size: 0,
    files: 0,
    path: ".rustup/downloads",
    consequence: "Arquivos temporários e downloads do Rustup.",
  },
  {
    id: "maven",
    name: "Repositório local Maven",
    app: "Maven",
    group: "Desenvolvimento",
    size: 0,
    files: 0,
    path: ".m2/repository",
    consequence: "Dependências Java em cache no repositório local.",
  },
  {
    id: "gobuild",
    name: "Build cache do Go",
    app: "Go",
    group: "Desenvolvimento",
    size: 0,
    files: 0,
    path: "AppData/Local/go-build",
    consequence: "Cache de compilação do Go. Recriado na próxima compilação.",
  },
  {
    id: "gomodcache",
    name: "Cache de módulos do Go",
    app: "Go",
    group: "Desenvolvimento",
    size: 0,
    files: 0,
    path: "go/pkg/mod/cache/download",
    consequence: "Módulos baixados do Go.",
  },
  {
    id: "pnpm-local",
    name: "Store do pnpm",
    app: "pnpm",
    group: "Desenvolvimento",
    size: 0,
    files: 0,
    path: "AppData/Local/pnpm/store",
    consequence: "Store local de pacotes do pnpm.",
  },
  {
    id: "pnpm-store",
    name: "Store do pnpm (home)",
    app: "pnpm",
    group: "Desenvolvimento",
    size: 0,
    files: 0,
    path: ".pnpm-store",
    consequence: "Store compartilhada de pacotes do pnpm.",
  },
  {
    id: "nuget",
    name: "Cache HTTP do NuGet",
    app: "NuGet",
    group: "Desenvolvimento",
    size: 0,
    files: 0,
    path: "AppData/Local/NuGet/v3-cache",
    consequence: "Cache de requisições HTTP e metadados do NuGet.",
  },
  {
    id: "nuget-home",
    name: "Pacotes baixados do NuGet",
    app: "NuGet",
    group: "Desenvolvimento",
    size: 0,
    files: 0,
    path: ".nuget/packages",
    consequence: "Pacotes NuGet baixados no diretório de usuário.",
  },
  {
    id: "offline",
    name: "Dados de sites offline",
    app: "Google Chrome",
    group: "Navegadores",
    size: 0,
    files: 0,
    path: "AppData/Local/Google/Chrome/User Data/Default/File System",
    consequence: "Podem conter documentos e trabalho offline. Não entram na limpeza automática.",
    blocked: true,
  },
];

export function mergeRulesWithScan(items: JunkItem[]): Rule[] {
  const map = new Map<string, JunkItem>();
  for (const item of items) map.set(item.id, item);

  const merged: Rule[] = BASE_RULES.map((rule) => {
    const item = map.get(rule.id);
    if (!item) return rule;
    return {
      ...rule,
      size: item.size,
      files: item.files,
      path: item.paths[0] || rule.path,
      caution: item.caution ?? rule.caution,
    };
  });

  // Include any extra junk items from the backend not explicitly in BASE_RULES
  for (const item of items) {
    if (!BASE_RULES.some((r) => r.id === item.id)) {
      merged.push({
        id: item.id,
        name: item.label,
        app: item.label,
        group: item.group,
        size: item.size,
        files: item.files,
        path: item.paths[0] || item.label,
        consequence: item.caution
          ? `${item.caution} Fica desmarcado até você escolher.`
          : "Caches e arquivos temporários que os aplicativos recriam sozinhos.",
        caution: item.caution ?? undefined,
      });
    }
  }

  return merged;
}

export type Settings = {
  excluded: string[];
  schedule: boolean;
  frequency: string;
  time: string;
  reminder: boolean;
};

export const DEFAULT_SETTINGS: Settings = {
  excluded: [],
  schedule: false,
  frequency: "Semanal",
  time: "18:00",
  reminder: true,
};

export const SETTINGS_KEY = "faxina-settings-v1";
export const HISTORY_KEY = "faxina-history-v1";

export type Report = {
  id: string;
  date: string;
  selected: number;
  removed: number;
  trashed: number;
  permanent: number;
  observed: number;
  skipped: number;
  files: number;
  destination: "trash" | "permanent";
};

function getStorage(): Storage | null {
  try {
    if (typeof window !== "undefined" && window.localStorage) return window.localStorage;
    if (typeof localStorage !== "undefined") return localStorage;
  } catch {
    return null;
  }
  return null;
}

export function readSettings(): Settings {
  try {
    const storage = getStorage();
    if (!storage) return DEFAULT_SETTINGS;
    const raw = storage.getItem(SETTINGS_KEY);
    if (!raw) return DEFAULT_SETTINGS;
    const value = JSON.parse(raw);
    if (!value || typeof value !== "object") return DEFAULT_SETTINGS;
    return {
      excluded: Array.isArray(value.excluded)
        ? value.excluded.filter((id: unknown): id is string => typeof id === "string")
        : [],
      schedule: value.schedule === true,
      frequency: ["Semanal", "Mensal"].includes(value.frequency) ? value.frequency : "Semanal",
      time: typeof value.time === "string" && /^([01]\d|2[0-3]):[0-5]\d$/.test(value.time) ? value.time : "18:00",
      reminder: value.reminder !== false,
    };
  } catch {
    return DEFAULT_SETTINGS;
  }
}

export function saveSettings(settings: Settings): boolean {
  try {
    const storage = getStorage();
    if (!storage) return false;
    storage.setItem(SETTINGS_KEY, JSON.stringify(settings));
    return true;
  } catch {
    return false;
  }
}

export function readHistory(): Report[] {
  try {
    const storage = getStorage();
    if (!storage) return [];
    const raw = storage.getItem(HISTORY_KEY);
    if (!raw) return [];
    const value = JSON.parse(raw);
    if (!Array.isArray(value)) return [];
    return value
      .filter(
        (r): r is Report =>
          Boolean(r) &&
          typeof r.id === "string" &&
          typeof r.date === "string" &&
          !Number.isNaN(Date.parse(r.date)) &&
          ["trash", "permanent"].includes(r.destination) &&
          [r.selected, r.removed, r.trashed, r.permanent, r.observed, r.skipped, r.files].every(
            (n) => typeof n === "number" && Number.isFinite(n) && n >= 0,
          ),
      )
      .slice(0, 12);
  } catch {
    return [];
  }
}

export function saveHistory(history: Report[]): boolean {
  try {
    const storage = getStorage();
    if (!storage) return false;
    storage.setItem(HISTORY_KEY, JSON.stringify(history.slice(0, 12)));
    return true;
  } catch {
    return false;
  }
}

export const gb = (value: number) => formatBytes(value);
export const count = (value: number) => formatCount(value);
export const totalSize = (rules: Rule[]) => rules.reduce((sum, r) => sum + r.size, 0);

const CLEANUP_INTERVAL_DAYS: Record<string, number> = { Semanal: 7, Mensal: 30 };

/**
 * When the next routine cleanup is worth a nudge: the last cleanup in the history plus the chosen
 * interval, moved to the chosen time of day. Null when the routine is off. The Faxina has no
 * background service, so this is a reminder shown when the app opens, not a scheduled task.
 */
export function nextCleanup(settings: Settings, history: Report[]): Date | null {
  if (!settings.schedule) return null;
  const days = CLEANUP_INTERVAL_DAYS[settings.frequency] ?? 7;
  const last = history
    .map((r) => Date.parse(r.date))
    .filter((t) => Number.isFinite(t))
    .sort((a, b) => b - a)[0];
  if (last === undefined) return null;
  const [hour, minute] = settings.time.split(":").map(Number);
  // Calendar addition in local time: adding fixed milliseconds across a DST change would move the
  // reminder by an hour and can even flip the day.
  const next = new Date(last);
  next.setDate(next.getDate() + days);
  next.setHours(hour || 0, minute || 0, 0, 0);
  return next;
}

export function isCleanupDue(settings: Settings, history: Report[], now: Date = new Date()): boolean {
  const next = nextCleanup(settings, history);
  return next !== null && now.getTime() >= next.getTime();
}

export function recommended(rules: Rule[], settings: Settings) {
  return rules.filter((r) => r.size > 0 && !r.blocked && !r.busy && !r.caution && !settings.excluded.includes(r.id));
}

export function createReport(
  rules: Rule[],
  destination: Report["destination"],
  actualFreed?: number,
  actualSkipped?: number,
  actualFiles?: number,
): Report {
  const done = rules.filter((r) => !r.busy && !r.blocked);
  const selected = totalSize(rules);
  const removed = actualFreed !== undefined ? actualFreed : totalSize(done);
  const skipped = actualSkipped !== undefined ? actualSkipped : rules.filter((r) => r.busy).length;
  const files = actualFiles !== undefined ? actualFiles : done.reduce((sum, r) => sum + r.files, 0);

  return {
    id: crypto.randomUUID(),
    date: new Date().toISOString(),
    selected,
    removed,
    trashed: destination === "trash" ? removed : 0,
    permanent: destination === "permanent" ? removed : 0,
    // Only a permanent delete frees disk space; the trash still occupies the volume.
    observed: destination === "permanent" ? removed : 0,
    skipped,
    files,
    destination,
  };
}
