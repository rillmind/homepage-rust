import type { Service, ServiceInput, Stats, Status } from "./types";

async function request<T>(
  path: string,
  init?: { method?: string; body?: unknown; signal?: AbortSignal },
): Promise<T> {
  const res = await fetch("/api" + path, {
    method: init?.method,
    signal: init?.signal,
    headers: init?.body ? { "Content-Type": "application/json" } : undefined,
    body: init?.body ? JSON.stringify(init.body) : undefined,
  });
  if (!res.ok) {
    const data = await res.json().catch(() => null);
    throw new Error(data?.error ?? `Erro ${res.status} no servidor`);
  }
  return res.status === 204 ? (undefined as T) : res.json();
}

export const getStats = (signal: AbortSignal) =>
  request<Stats>("/stats", { signal });

export const getStatuses = (signal: AbortSignal) =>
  request<Record<number, Status>>("/services/status", { signal });

export const getServices = () => request<Service[]>("/services");

export const saveService = (input: ServiceInput, id?: number) =>
  id === undefined
    ? request<Service>("/services", { method: "POST", body: input })
    : request<Service>(`/services/${id}`, { method: "PUT", body: input });

export const deleteService = (id: number) =>
  request<void>(`/services/${id}`, { method: "DELETE" });

export const probePort = (port: number) => request<Status>(`/probe/${port}`);
