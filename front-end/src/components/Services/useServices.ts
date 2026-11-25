import { useEffect, useState } from "react";
import {
  ApiError,
  deleteService,
  errorText,
  getServices,
  getStatuses,
  runContainer,
  saveService,
} from "../../lib/api";
import type { ContainerAction, Service, ServiceInput } from "../../lib/types";
import { usePolling } from "../../lib/usePolling";

const STATUS_INTERVAL = 10_000;
const RETRY_INTERVAL = 3_000;

const NOTICE_TIME = 6_000;

export type Notice = {
  id: number;
  kind: "ok" | "warning" | "failed";
  message: string;
  details: string;
};

export function useServices() {
  const [services, setServices] = useState<Service[] | null>(null);
  const [busy, setBusy] = useState<Record<number, ContainerAction>>({});
  const [notices, setNotices] = useState<Notice[]>([]);
  const [finished, setFinished] = useState(0);
  const statuses = usePolling(
    getStatuses,
    STATUS_INTERVAL,
    `${services?.map((item) => `${item.id}:${item.port}`)}|${finished}`,
  );

  useEffect(() => {
    let active = true;
    let timer: ReturnType<typeof setTimeout> | undefined;

    const load = () =>
      getServices().then(
        (list) => active && setServices(list),
        () => {
          if (active) timer = setTimeout(load, RETRY_INTERVAL);
        },
      );

    load();
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, []);

  const save = async (input: ServiceInput, id?: number) => {
    const saved = await saveService(input, id);
    setServices((list) =>
      id === undefined
        ? [...(list ?? []), saved]
        : (list ?? []).map((item) => (item.id === id ? saved : item)),
    );
  };

  const remove = async (id: number) => {
    await deleteService(id);
    setServices((list) => (list ?? []).filter((item) => item.id !== id));
  };

  const dismiss = (id: number) =>
    setNotices((list) => list.filter((item) => item.id !== id));

  // Sucesso some sozinho; aviso e erro ficam até serem fechados.
  const notify = (notice: Omit<Notice, "id">) => {
    const id = Date.now() + Math.random();
    setNotices((list) => [...list, { ...notice, id }]);
    if (notice.kind === "ok") setTimeout(() => dismiss(id), NOTICE_TIME);
  };

  const run = async (service: Service, action: ContainerAction) => {
    setBusy((all) => ({ ...all, [service.id]: action }));
    try {
      const { message, warning, details } = await runContainer(
        service.id,
        action,
      );
      notify({ kind: warning ? "warning" : "ok", message, details });
    } catch (error) {
      const verb = {
        start: "iniciar",
        restart: "reiniciar",
        update: "atualizar",
      }[action];
      notify({
        kind: "failed",
        message: `Não foi possível ${verb} ${service.name}: ${errorText(error)}`,
        details: error instanceof ApiError ? error.details : "",
      });
    }
    setBusy((all) => {
      const others = { ...all };
      delete others[service.id];
      return others;
    });
    setFinished((n) => n + 1);
  };

  return { services, statuses, save, remove, run, busy, notices, dismiss };
}

export type ServicesState = ReturnType<typeof useServices>;
