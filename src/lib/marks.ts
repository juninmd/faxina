import type { Kind } from "./api";

export interface Mark {
  path: string;
  name: string;
  size: number;
  kind: Kind;
}

export type Marks = ReadonlyMap<string, Mark>;
