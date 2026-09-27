export type Swallow = (
  keys: Iterable<string>,
  action: () => Promise<{ freed: number; failed: number }>,
  after: () => void,
) => Promise<void>;
