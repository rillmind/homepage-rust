import { useEffect, useState } from "react";

/**
 * Busca de novo só depois que a resposta anterior chega, e para com a aba escondida.
 * `fetcher` precisa ser estável. `key` força uma busca imediata quando muda.
 */
export function usePolling<T>(
  fetcher: (signal: AbortSignal) => Promise<T>,
  intervalMs: number,
  key?: unknown,
) {
  const [data, setData] = useState<T | null>(null);

  useEffect(() => {
    const abort = new AbortController();
    let timer: ReturnType<typeof setTimeout> | undefined;
    let running = false;

    const run = async () => {
      if (running || document.hidden) return;
      clearTimeout(timer);
      running = true;
      try {
        const next = await fetcher(abort.signal);
        if (!abort.signal.aborted) setData(next);
      } catch {
        if (!abort.signal.aborted) setData(null); // servidor fora do ar
      }
      running = false;
      if (!abort.signal.aborted) timer = setTimeout(run, intervalMs);
    };

    run();
    document.addEventListener("visibilitychange", run);
    return () => {
      abort.abort();
      clearTimeout(timer);
      document.removeEventListener("visibilitychange", run);
    };
  }, [fetcher, intervalMs, key]);

  return data;
}
