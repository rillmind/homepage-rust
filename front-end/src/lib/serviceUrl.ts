import type { ServiceInput } from "./types";

/** Usa o host da página, para o link valer tanto na rede local quanto por VPN. */
export function serviceUrl(protocol: ServiceInput["protocol"], port: number) {
  return `${protocol}://${window.location.hostname}:${port}`;
}
