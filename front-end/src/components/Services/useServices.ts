import { useEffect, useState } from "react";
import {
  deleteService,
  getServices,
  getStatuses,
  saveService,
} from "../../lib/api";
import type { Service, ServiceInput } from "../../lib/types";
import { usePolling } from "../../lib/usePolling";

const STATUS_INTERVAL = 10_000;
const RETRY_INTERVAL = 3_000;

export function useServices() {
  const [services, setServices] = useState<Service[] | null>(null);
  const statuses = usePolling(getStatuses, STATUS_INTERVAL, services);

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

  return { services, statuses, save, remove };
}

export type ServicesState = ReturnType<typeof useServices>;
