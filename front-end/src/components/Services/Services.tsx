import { useEffect, useRef, useState } from "react";
import { normalize } from "../../lib/format";
import { icons } from "../../lib/icons";
import type { Service } from "../../lib/types";
import ServiceCard from "./ServiceCard";
import ServiceForm from "./ServiceForm";
import type { ServicesState } from "./useServices";
import "./Services.css";

type Props = ServicesState & { searchRequest: number };

export default function Services({
  services,
  statuses,
  save,
  remove,
  searchRequest,
}: Props) {
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState<string | null>(null);
  const [editing, setEditing] = useState<Service | "new" | null>(null);
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
  const visible = services.filter((item) => {
    const text = normalize(
      `${item.name} ${item.description} ${item.category} :${item.port}`,
    );
    return (
      (!active || item.category === active) &&
      words.every((word) => text.includes(word))
    );
  });

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
            onEdit={() => setEditing(item)}
          />
        ))}
        <button className="service serviceAdd" onClick={() => setEditing("new")}>
          <span className="serviceIcon">{icons.add}</span>
          <strong>Adicionar Serviço</strong>
          <span>Conectar novo contêiner ou porta</span>
        </button>
      </section>

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
