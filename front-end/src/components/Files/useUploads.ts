import { useEffect, useRef, useState } from "react";
import { errorText, uploadFile } from "../../lib/api";

export type Upload = {
  id: number;
  name: string;
  total: number;
  sent: number;
  state: "waiting" | "running" | "done" | "failed";
  error: string;
};

type Queued = { id: number; dir: string; file: File };

/** Envia um arquivo por vez. */
export function useUploads(onDone: (dir: string) => void) {
  const [uploads, setUploads] = useState<Upload[]>([]);
  const queue = useRef<Queued[]>([]);
  const running = useRef<{ id: number; abort: AbortController } | null>(null);
  const nextId = useRef(1);
  const done = useRef(onDone);

  useEffect(() => {
    done.current = onDone;
  }, [onDone]);

  const active = uploads.some(
    (item) => item.state === "waiting" || item.state === "running",
  );
  useEffect(() => {
    if (!active) return;
    const warn = (event: BeforeUnloadEvent) => event.preventDefault();
    window.addEventListener("beforeunload", warn);
    return () => window.removeEventListener("beforeunload", warn);
  }, [active]);

  // Sair da tela cancela o que estava na fila.
  useEffect(
    () => () => {
      queue.current = [];
      running.current?.abort.abort();
    },
    [],
  );

  const change = (id: number, values: Partial<Upload>) =>
    setUploads((list) =>
      list.map((item) => (item.id === id ? { ...item, ...values } : item)),
    );

  const next = async () => {
    const item = running.current ? undefined : queue.current.shift();
    if (!item) return;
    const abort = new AbortController();
    running.current = { id: item.id, abort };
    change(item.id, { state: "running" });
    try {
      await uploadFile(item.dir, item.file, abort.signal, (sent) =>
        change(item.id, { sent }),
      );
      change(item.id, { state: "done", sent: item.file.size });
      done.current(item.dir);
    } catch (error) {
      change(item.id, { state: "failed", error: errorText(error) });
    }
    running.current = null;
    next();
  };

  const add = (dir: string, files: File[]) => {
    const added = files.map((file) => ({ id: nextId.current++, dir, file }));
    queue.current.push(...added);
    setUploads((list) => [
      ...list,
      ...added.map(({ id, file }) => ({
        id,
        name: file.name,
        total: file.size,
        sent: 0,
        state: "waiting" as const,
        error: "",
      })),
    ]);
    next();
  };

  const cancel = (id: number) => {
    if (running.current?.id === id) return running.current.abort.abort();
    queue.current = queue.current.filter((item) => item.id !== id);
    change(id, { state: "failed", error: "Cancelado" });
  };

  const clear = () =>
    setUploads((list) =>
      list.filter(
        (item) => item.state === "waiting" || item.state === "running",
      ),
    );

  return { uploads, add, cancel, clear };
}
