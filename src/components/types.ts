export type Swallow = (
  keys: Iterable<string>,
  action: () => Promise<{ freed: number; failed: number; reason?: string }>,
  after: () => void | Promise<void>,
) => Promise<void>;
