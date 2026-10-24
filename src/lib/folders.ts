import type { ViewNode } from "./api";
import { formatAge, formatBytes } from "./format";

export interface FolderInfo {
  whatItIs: string;
  ifRemoved: string;
}

const SYNC = "Se o OneDrive ou outro serviço sincroniza esta pasta, apagar aqui apaga nos outros aparelhos também.";

/** Plain-language notes for folders that hold many things, matched by lowercase folder name. */
const BY_NAME: Record<string, FolderInfo> = {
  downloads: {
    whatItIs: "Arquivos baixados da internet. Costuma guardar instaladores e zips antigos.",
    ifRemoved: "O que você ainda precisa some: olhe o conteúdo antes de apagar.",
  },
  desktop: { whatItIs: "Tudo o que está na sua área de trabalho.", ifRemoved: `São seus arquivos. ${SYNC}` },
  documents: { whatItIs: "Seus documentos e arquivos salvos por você.", ifRemoved: `São seus arquivos. ${SYNC}` },
  pictures: { whatItIs: "Suas fotos e imagens.", ifRemoved: `São seus arquivos. ${SYNC}` },
  videos: { whatItIs: "Seus vídeos.", ifRemoved: `São seus arquivos. ${SYNC}` },
  movies: { whatItIs: "Seus vídeos.", ifRemoved: `São seus arquivos. ${SYNC}` },
  music: { whatItIs: "Suas músicas.", ifRemoved: `São seus arquivos. ${SYNC}` },
  onedrive: {
    whatItIs: "Seus arquivos do OneDrive neste computador.",
    ifRemoved: "Apagar aqui remove também da nuvem e dos outros aparelhos. Use “Liberar espaço” do OneDrive.",
  },
  appdata: {
    whatItIs: "Configurações e dados que os aplicativos guardam para você. Quase ninguém abre esta pasta.",
    ifRemoved: "Aplicativos perdem configurações e dados salvos. Não apague: abra e veja qual app usa o espaço.",
  },
  programdata: {
    whatItIs: "Dados compartilhados por todos os usuários e instalados por programas.",
    ifRemoved: "Programas podem parar de funcionar. Limpe pelo próprio programa.",
  },
  "program files": {
    whatItIs: "Programas instalados neste computador.",
    ifRemoved: "Os programas deixam de abrir. Desinstale pelo Windows em vez de apagar.",
  },
  "program files (x86)": {
    whatItIs: "Programas de 32 bits instalados neste computador.",
    ifRemoved: "Os programas deixam de abrir. Desinstale pelo Windows em vez de apagar.",
  },
  windows: {
    whatItIs: "O sistema operacional.",
    ifRemoved: "O computador pode não iniciar mais. Nunca apague.",
  },
  winsxs: {
    whatItIs: "Componentes e versões antigas do Windows, usados nas atualizações.",
    ifRemoved: "Pode quebrar o Windows. Libere espaço pela Limpeza de Disco, nunca à mão.",
  },
  "$recycle.bin": {
    whatItIs: "A Lixeira do disco: arquivos que você já apagou mas ainda ocupam espaço.",
    ifRemoved: "Esvazie pela Lixeira. Os arquivos não poderão ser restaurados depois.",
  },
  library: {
    whatItIs: "Configurações e dados dos aplicativos no macOS.",
    ifRemoved: "Aplicativos perdem configurações e dados salvos. Não apague.",
  },
  ".git": {
    whatItIs: "O histórico de versões de um projeto.",
    ifRemoved: "O projeto perde todo o histórico. Só apague se ele estiver salvo em outro lugar.",
  },
  node_modules: {
    whatItIs: "Dependências de um projeto JavaScript, baixadas pelo npm, bun ou yarn.",
    ifRemoved: "Reinstale com `npm install` (ou `bun install`) quando precisar do projeto.",
  },
  target: {
    whatItIs: "Saída de compilação de um projeto Rust (ou Java).",
    ifRemoved: "A próxima compilação demora mais, mas recria tudo.",
  },
  ".gradle": {
    whatItIs: "Cache de dependências e compilações do Gradle.",
    ifRemoved: "O próximo build baixa tudo de novo.",
  },
  ".m2": {
    whatItIs: "Repositório local de dependências do Maven.",
    ifRemoved: "O próximo build baixa tudo de novo.",
  },
  ".cargo": {
    whatItIs: "Pacotes e ferramentas do Rust baixados pelo cargo.",
    ifRemoved: "Ferramentas instaladas somem; os pacotes são baixados de novo.",
  },
  ".rustup": {
    whatItIs: "As versões do compilador Rust instaladas.",
    ifRemoved: "É preciso reinstalar o Rust.",
  },
  ".npm": { whatItIs: "Cache de pacotes do npm.", ifRemoved: "Os pacotes são baixados de novo quando precisar." },
  ".cache": {
    whatItIs: "Caches que aplicativos guardam para ganhar velocidade.",
    ifRemoved: "Os aplicativos recriam o cache, mas ficam mais lentos na primeira vez.",
  },
};

/** `local`, `roaming` and `locallow` only mean something inside AppData. */
const APP_DATA_CHILDREN: Record<string, FolderInfo> = {
  local: {
    whatItIs: "Dados dos aplicativos que ficam só neste computador: caches, bancos e arquivos temporários.",
    ifRemoved: "Aplicativos perdem dados e configurações. Abra e veja qual deles ocupa mais.",
  },
  roaming: {
    whatItIs: "Configurações dos aplicativos que acompanham a sua conta.",
    ifRemoved: "Aplicativos perdem configurações salvas.",
  },
  locallow: {
    whatItIs: "Dados de aplicativos com permissões restritas, como navegadores e jogos.",
    ifRemoved: "Aplicativos perdem dados salvos.",
  },
  temp: {
    whatItIs: "Arquivos temporários que programas deixam para trás.",
    ifRemoved: "Em geral é seguro, mas programas abertos podem estar usando alguns.",
  },
};

function segments(path: string): string[] {
  return path.split(/[\\/]/).filter(Boolean);
}

// Maps, not plain lookups: a folder called `constructor` must not resolve to an Object.prototype member.
const BY_NAME_MAP = new Map(Object.entries(BY_NAME));
const APP_DATA_MAP = new Map(Object.entries(APP_DATA_CHILDREN));

export function explainFolder(path: string): FolderInfo | null {
  const parts = segments(path).map((p) => p.toLowerCase());
  const name = parts[parts.length - 1];
  if (!name) return null;
  const inAppData = parts[parts.length - 2] === "appdata" || (name === "temp" && parts[parts.length - 3] === "appdata");
  if (inAppData && APP_DATA_MAP.has(name)) return APP_DATA_MAP.get(name) ?? null;
  return BY_NAME_MAP.get(name) ?? null;
}

const PROMPT_ENTRIES = 15;

/** Text to paste into an AI assistant: what the folder is and its biggest entries, nothing else. */
export function assistantPrompt(node: ViewNode, system: string): string {
  const entries = [...node.children]
    .filter((c) => !c.grouped)
    .sort((a, b) => b.size - a.size)
    .slice(0, PROMPT_ENTRIES)
    .map((c) => `- ${c.name}${c.isDir ? "/" : ""}: ${formatBytes(c.size)}`);
  return [
    `Estou liberando espaço em disco no ${system} e quero entender uma pasta antes de apagar qualquer coisa.`,
    "",
    `Pasta: ${node.path}`,
    `Tamanho: ${formatBytes(node.size)} em ${node.files.toLocaleString("pt-BR")} arquivos`,
    `Última escrita: ${formatAge(node.modified)}`,
    "",
    entries.length ? "Maiores itens dentro dela:" : "Não há itens grandes dentro dela.",
    ...entries,
    "",
    "Explique o que é essa pasta, o que usa tanto espaço e o que é seguro apagar. Se não tiver certeza, diga o que eu devo verificar. Não sugira apagar nada que eu não possa recuperar.",
  ].join("\n");
}
