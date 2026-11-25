import type { Service, Status } from "../../lib/types";

export const SORTS = {
  custom: "Personalizada",
  az: "Nome (A-Z)",
  za: "Nome (Z-A)",
  category: "Categoria",
  online: "Online primeiro",
  offline: "Offline primeiro",
  port: "Porta",
  newest: "Mais recentes",
};

export type Sort = keyof typeof SORTS;

const KEY = "homepage-services-sort";

export function loadSort(): Sort {
  try {
    const saved = localStorage.getItem(KEY);
    return saved && saved in SORTS ? (saved as Sort) : "custom";
  } catch {
    return "custom";
  }
}

export function storeSort(sort: Sort) {
  try {
    localStorage.setItem(KEY, sort);
  } catch {
    // Sem armazenamento, vale só até fechar a página.
  }
}

const { compare } = new Intl.Collator("pt-BR", { numeric: true });

export function sortServices(
  services: Service[],
  sort: Sort,
  statuses: Record<number, Status> | null,
) {
  const online = (item: Service) =>
    Number(statuses?.[item.id]?.online ?? false);
  const byName = (a: Service, b: Service) => compare(a.name, b.name);
  const orders: Record<Sort, (a: Service, b: Service) => number> = {
    custom: () => 0,
    az: byName,
    za: (a, b) => byName(b, a),
    category: (a, b) => compare(a.category, b.category) || byName(a, b),
    online: (a, b) => online(b) - online(a) || byName(a, b),
    offline: (a, b) => online(a) - online(b) || byName(a, b),
    port: (a, b) => a.port - b.port,
    newest: (a, b) => b.id - a.id,
  };
  return services.toSorted(orders[sort]);
}
