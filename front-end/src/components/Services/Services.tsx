import { useEffect, useRef, useState } from "react";
import { normalize } from "../../lib/format";
import { icons } from "../../lib/icons";
import type { Service } from "../../lib/types";
import CasaImport from "./CasaImport";
import ServiceCard from "./ServiceCard";
import ServiceMenu, { type MenuTarget } from "./ServiceMenu";
import ServiceForm from "./ServiceForm";
import { loadSort, SORTS, sortServices, storeSort, type Sort } from "./sort";
import type { ServicesState } from "./useServices";
import "./Services.css";

type Props = ServicesState & { searchRequest: number };

export default function Services({
  services,
  statuses,
  save,
  remove,
  reorder,
  run,
  busy,
  searchRequest,
}: Props) {
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState<string | null>(null);
  const [editing, setEditing] = useState<Service | "new" | null>(null);
  const [importing, setImporting] = useState(false);
  const [menu, setMenu] = useState<MenuTarget | null>(null);
  const [sort, setSort] = useState(loadSort);
  const [dragging, setDragging] = useState<number | null>(null);
  const [over, setOver] = useState<number | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (searchRequest > 0) searchRef.current?.focus();
  }, [searchRequest]);

  if (services === null) {
    return <p className="message">Carregando serviços...</p>;
  }

  const categories = [...new Set(services.map((item) => item.category))].sort(
    (a, b) => a.localeCompare(b, "pt-BR"),
  );
  // A categoria some se o último serviço dela for excluído.
  const active = category && categories.includes(category) ? category : null;
  const words = normalize(query).split(/\s+/);
  const visible = sortServices(services, sort, statuses).filter((item) => {
    const text = normalize(
      `${item.name} ${item.description} ${item.category} :${item.port}`,
    );
    return (
      (!active || item.category === active) &&
      words.every((word) => text.includes(word))
    );
  });

  /** Põe o serviço no lugar do outro, na lista completa e não só na filtrada. */
  const move = (id: number, targetId: number) => {
    const from = services.findIndex((item) => item.id === id);
    const to = services.findIndex((item) => item.id === targetId);
    if (from < 0 || to < 0 || from === to) return;
    const next = services.toSpliced(from, 1);
    next.splice(to, 0, services[from]);
    reorder(next);
  };

  const menuIndex = menu
    ? visible.findIndex((item) => item.id === menu.service.id)
    : -1;
  const neighbor = (step: number) => {
    const other = menuIndex < 0 ? undefined : visible[menuIndex + step];
    if (sort !== "custom" || !menu || !other) return undefined;
    return () => move(menu.service.id, other.id);
  };

  return (
    <>
      <section className="toolbar" aria-label="Filtros de serviços">
        <label className="search">
          <span>{icons.search}</span>
          <input
            ref={searchRef}
            type="search"
            aria-label="Pesquisar serviços"
            placeholder="Pesquisar serviços..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <kbd>Ctrl K</kbd>
        </label>
        <div className="filters">
          {[null, ...categories].map((name) => (
            <button
              key={name ?? ""}
              className={name === active ? "filter active" : "filter"}
              aria-pressed={name === active}
              onClick={() => setCategory(name)}
            >
              {name ?? `Todos (${services.length})`}
            </button>
          ))}
        </div>
        <label className="sort">
          {icons.sort}
          <select
            aria-label="Ordenar serviços"
            value={sort}
            onChange={(e) => {
              const next = e.target.value as Sort;
              setSort(next);
              storeSort(next);
            }}
          >
            {Object.entries(SORTS).map(([key, label]) => (
              <option key={key} value={key}>
                {label}
              </option>
            ))}
          </select>
        </label>
        <button
          className="button importButton"
          aria-label="Importar do CasaOS"
          title="Importar do CasaOS"
          onClick={() => setImporting(true)}
        >
          {icons.import} Importar
        </button>
      </section>

      {services.length > 0 && visible.length === 0 && (
        <p className="message">Nenhum serviço encontrado.</p>
      )}

      <section className="services" aria-label="Serviços">
        {visible.map((item) => (
          <ServiceCard
            key={item.id}
            service={item}
            status={statuses?.[item.id]}
            busy={busy[item.id]}
            drag={
              sort === "custom"
                ? {
                    active: dragging === item.id,
                    over: over === item.id && dragging !== item.id,
                    onStart: () => setDragging(item.id),
                    onOver: () => setOver(item.id),
                    onDrop: () => {
                      if (dragging !== null) move(dragging, item.id);
                    },
                    onEnd: () => {
                      setDragging(null);
                      setOver(null);
                    },
                  }
                : undefined
            }
            onEdit={() => setEditing(item)}
            onMenu={(x, y) => setMenu({ service: item, x, y })}
          />
        ))}
        <button
          className="service serviceAdd"
          onClick={() => setEditing("new")}
        >
          <span className="serviceIcon">{icons.add}</span>
          <strong>Adicionar Serviço</strong>
          <span>Conectar novo contêiner ou porta</span>
        </button>
      </section>

      {menu && (
        <ServiceMenu
          key={menu.service.id}
          target={menu}
          busy={menu.service.id in busy}
          onRun={(action) => run(menu.service, action)}
          onBefore={neighbor(-1)}
          onAfter={neighbor(1)}
          onEdit={() => setEditing(menu.service)}
          onClose={() => setMenu(null)}
        />
      )}

      {importing && (
        <CasaImport
          services={services}
          onSave={save}
          onClose={() => setImporting(false)}
        />
      )}

      {editing && (
        <ServiceForm
          service={editing === "new" ? undefined : editing}
          categories={categories}
          onSave={save}
          onRemove={remove}
          onClose={() => setEditing(null)}
        />
      )}
    </>
  );
}
