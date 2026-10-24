import { useEffect, useRef, useState } from "react";
import { countValue } from "../lib/ease";
import { shouldAnimate } from "../lib/motion";

const DURATION = 700;

/** Shows `format(value)`, counting from the last shown value whenever `value` changes. */
export default function CountUp({ value, format }: { value: number; format: (n: number) => string }) {
  const [shown, setShown] = useState(value);
  const from = useRef(value);

  useEffect(() => {
    if (!shouldAnimate() || from.current === value) {
      from.current = value;
      setShown(value);
      return;
    }
    const start = performance.now();
    const origin = from.current;
    let frame = 0;
    const tick = (now: number) => {
      const v = countValue(origin, value, now - start, DURATION);
      from.current = v;
      setShown(v);
      if (v !== value) frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [value]);

  return <>{format(shown)}</>;
}
