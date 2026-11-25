import type {
  AuthStatus,
  CasaApp,
  Container,
  ContainerAction,
  Found,
  Job,
  Listing,
  Outcome,
  Service,
  ServiceInput,
  Stats,
  Status,
  Trashed,
} from "./types";

const BASE = "/homepage-api";

/** Disparado quando a sessão expira. */
export const SESSION_LOST = "homepage-session-lost";

/** `details` é o log do contêiner, quando há. */
export class ApiError extends Error {
  details: string;

  constructor(message: string, details: string) {
    super(message);
    this.details = details;
  }
}

type Init = { method?: string; body?: unknown; signal?: AbortSignal };

async function request<T>(path: string, init: Init = {}): Promise<T> {
  const res = await fetch(BASE + path, {
    method: init.method,
    signal: init.signal,
    headers: {
      "X-Homepage": "1",
      ...(init.body !== undefined && { "Content-Type": "application/json" }),
    },
    body: init.body === undefined ? undefined : JSON.stringify(init.body),
  });
  if (!res.ok) {
    if (res.status === 401 && !path.startsWith("/auth/")) {
      window.dispatchEvent(new Event(SESSION_LOST));
    }
    const data = await res.json().catch(() => null);
    throw new ApiError(
      data?.error ?? `Erro ${res.status} no servidor`,
      data?.details ?? "",
    );
  }
  const text = await res.text();
  return (text ? JSON.parse(text) : undefined) as T;
}

const get = <T>(path: string, signal?: AbortSignal) =>
  request<T>(path, { signal });
const post = <T = void>(path: string, body: unknown = {}) =>
  request<T>(path, { method: "POST", body });

const query = (params: Record<string, string>) =>
  "?" + new URLSearchParams(params).toString();

export function errorText(error: unknown) {
  return error instanceof Error ? error.message : "Erro inesperado";
}

export const getAuth = () => get<AuthStatus>("/auth/status");
export const register = (username: string, password: string) =>
  post("/auth/register", { username, password });
export const login = (username: string, password: string) =>
  post("/auth/login", { username, password });
export const logout = () => post("/auth/logout");
export const changePassword = (current: string, next: string) =>
  post("/auth/password", { current, new: next });

export const getStats = (signal: AbortSignal) => get<Stats>("/stats", signal);
export const getStatuses = (signal: AbortSignal) =>
  get<Record<number, Status>>("/services/status", signal);
export const getServices = () => get<Service[]>("/services");
export const saveService = (input: ServiceInput, id?: number) =>
  id === undefined
    ? post<Service>("/services", input)
    : request<Service>(`/services/${id}`, { method: "PUT", body: input });
export const orderServices = (ids: number[]) =>
  request<void>("/services/order", { method: "PUT", body: ids });
export const deleteService = (id: number) =>
  request<void>(`/services/${id}`, { method: "DELETE" });
export const getContainer = (id: number, signal: AbortSignal) =>
  get<Container>(`/services/${id}/container`, signal);
export const runContainer = (id: number, action: ContainerAction) =>
  post<Outcome>(`/services/${id}/${action}`);
export const probePort = (port: number) => get<Status>(`/probe/${port}`);
export const getCasaApps = () => get<CasaApp[]>("/casaos/apps");

export const listFiles = (path: string, signal?: AbortSignal) =>
  get<Listing>("/files/list" + query({ path }), signal);
export const searchFiles = (path: string, q: string, signal: AbortSignal) =>
  get<{ found: Found[]; partial: boolean }>(
    "/files/search" + query({ path, q }),
    signal,
  );
export const createItem = (
  kind: "folder" | "file",
  dir: string,
  name: string,
) => post(`/files/${kind}`, { dir, name });
export const renameItem = (path: string, name: string) =>
  post("/files/rename", { path, name });
export const transferItems = (
  mode: "copy" | "move",
  items: string[],
  to: string,
) => post("/files/transfer", { mode, items, to });
export const getJobs = (signal?: AbortSignal) =>
  get<Job[]>("/files/jobs", signal);
export const cancelJob = (id: number) =>
  request<void>(`/files/jobs/${id}`, { method: "DELETE" });
export const trashItems = (items: string[]) => post("/files/trash", { items });
export const getTrash = () => get<Trashed[]>("/files/trash");
export const restoreItem = (id: string) => post(`/files/trash/${id}/restore`);
export const purgeItems = (items: string[]) =>
  request<void>("/files/trash", { method: "DELETE", body: { items } });
export const readText = (path: string) =>
  get<{ content: string; version: string }>("/files/text" + query({ path }));
export const saveText = (path: string, content: string, version: string) =>
  request<{ version: string }>("/files/text", {
    method: "PUT",
    body: { path, content, version },
  });

export const downloadUrl = (path: string, inline = false) =>
  BASE + "/files/download" + query({ path, inline: String(inline) });
export const thumbUrl = (path: string, modified: number) =>
  BASE + "/files/thumb" + query({ path, v: String(modified) });

/** XHR porque o fetch não informa o progresso do envio. */
export function uploadFile(
  dir: string,
  file: File,
  signal: AbortSignal,
  onProgress: (sent: number) => void,
) {
  return new Promise<void>((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.open("POST", BASE + "/files/upload" + query({ dir, name: file.name }));
    xhr.setRequestHeader("X-Homepage", "1");
    xhr.upload.onprogress = (event) => onProgress(event.loaded);
    xhr.onload = () => {
      if (xhr.status === 201) return resolve();
      if (xhr.status === 401) window.dispatchEvent(new Event(SESSION_LOST));
      let message = `Erro ${xhr.status} no servidor`;
      try {
        message = JSON.parse(xhr.responseText).error ?? message;
      } catch {
        // Resposta sem JSON fica com a mensagem padrão.
      }
      reject(new Error(message));
    };
    xhr.onerror = () => reject(new Error("Falha na conexão"));
    xhr.onabort = () => reject(new Error("Cancelado"));
    signal.addEventListener("abort", () => xhr.abort());
    xhr.send(file);
  });
}
