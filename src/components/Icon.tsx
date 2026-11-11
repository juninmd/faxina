export type IconName =
  | "spark"
  | "shield"
  | "grid"
  | "check"
  | "clock"
  | "arrow"
  | "box"
  | "refresh"
  | "download"
  | "trash"
  | "search"
  | "close"
  | "file"
  | "copy"
  | "git"
  | "orbit"
  | "gear"
  | "command";

const paths: Record<IconName, string> = {
  spark: "m12 3 2.5 6.5L21 12l-6.5 2.5L12 21l-2.5-6.5L3 12l6.5-2.5L12 3Z",
  shield: "M12 3 4 6v6c0 5 8 9 8 9s8-4 8-9V6l-8-3Zm-4 9 3 3 5-6",
  grid: "M3 3h7v7H3V3Zm11 0h7v7h-7V3ZM3 14h7v7H3v-7Zm11 0h7v7h-7v-7Z",
  check: "m5 12 4 4L19 6",
  clock: "M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0ZM12 7v5l3 2",
  arrow: "M4 12h15m-6-6 6 6-6 6",
  box: "m12 3 9 5-9 5-9-5 9-5ZM3 8v9l9 5 9-5V8m-9 5v9",
  refresh: "M20 9a8 8 0 0 0-14-4L3 8m0-5v5h5m-4 7a8 8 0 0 0 14 4l3-3m0 5v-5h-5",
  download: "M12 3v12m-5-5 5 5 5-5M4 16v5h16v-5",
  trash: "M3 6h18M9 6V3h6v3M5 6l1 15h12l1-15M10 10v7m4-7v7",
  search: "M17 10a7 7 0 1 1-14 0 7 7 0 0 1 14 0Zm-2 5 6 6",
  close: "m6 6 12 12M6 18 18 6",
  file: "M5 3h9l5 5v13H5V3Zm9 0v6h5M8 13h8m-8 4h6",
  copy: "M8 4v12a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V7.2l-4.5-4.5H10a2 2 0 0 0-2 2Zm-4 4v12a2 2 0 0 0 2 2h12",
  orbit: "M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0ZM2.6 15.4A10 4-20 1 0 21.4 8.6 10 4-20 1 0 2.6 15.4Z",
  gear: "M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0ZM12 2v3m0 14v3M2 12h3m14 0h3M4.9 4.9 7 7m10 10 2.1 2.1M4.9 19.1 7 17M17 7l2.1-2.1",
  command: "M6 9h12M6 15h12M10 4 8 20m8-16-2 16",
  git: "M6 3v12M21 6a3 3 0 1 1-6 0 3 3 0 0 1 6 0ZM9 18a3 3 0 1 1-6 0 3 3 0 0 1 6 0ZM18 9a9 9 0 0 1-9 9",
};

export default function Icon({ name, size = 20 }: { name: IconName; size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d={paths[name]} />
    </svg>
  );
}
