export type Interface = {
  name: string;
  up: boolean;
  primary: boolean;
  ips: string[];
  rx: number;
  tx: number;
  speed: number | null;
};

export type Stats = {
  hostname: string | null;
  ip: string | null;
  date: string;
  time: string;
  uptime: number | null;
  cpu: { mhz: number | null; percent: number; cores: number } | null;
  memory: { total: number; used: number } | null;
  disk: { total: number; used: number; free: number; percent: number } | null;
  temp: {
    celsius: number | null;
    critical: number | null;
    fan: number | null;
    fanMax: number | null;
  };
  network: Interface[];
};

export type ServiceInput = {
  name: string;
  icon: string;
  protocol: "http" | "https";
  port: number;
  category: string;
  description: string;
  container: string;
};

export type Service = ServiceInput & { id: number };

export type Status = { online: boolean; ms: number | null };

export type Container = {
  name: string;
  running: boolean;
  compose: boolean;
  own: boolean;
};

export type ContainerAction = "start" | "restart" | "update";

export type Outcome = { message: string; warning: boolean; details: string };

export type CasaApp = Omit<ServiceInput, "category" | "container">;

export type AuthStatus = { username: string | null; authenticated: boolean };

export type Entry = {
  name: string;
  isDir: boolean;
  isLink: boolean;
  size: number;
  modified: number;
};

export type Found = Entry & { path: string };

export type Listing = { path: string; readOnly: boolean; entries: Entry[] };

export type Trashed = {
  id: string;
  original: string;
  deleted: number;
  isDir: boolean;
};

export type Job = {
  id: number;
  label: string;
  state: "waiting" | "running" | "done" | "failed" | "cancelled";
  error: string | null;
  done: number;
  total: number;
};
