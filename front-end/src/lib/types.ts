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
};

export type Service = ServiceInput & { id: number };

export type Status = { online: boolean; ms: number | null };
