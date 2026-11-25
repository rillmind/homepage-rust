import { categoryColor } from "../../lib/categories";
import { icons } from "../../lib/icons";
import { serviceUrl } from "../../lib/serviceUrl";
import type { ContainerAction, Service, Status } from "../../lib/types";
import ServiceIcon from "./ServiceIcon";

type Props = {
  service: Service;
  status?: Status;
  busy?: ContainerAction;
  onEdit: () => void;
  onMenu: (x: number, y: number) => void;
};

export default function ServiceCard({
  service,
  status,
  busy,
  onEdit,
  onMenu,
}: Props) {
  const state = busy
    ? "busy"
    : status && (status.online ? "online" : "offline");
  const label = {
    online: "Online",
    offline: "Offline",
    busy: { start: "Iniciando", restart: "Reiniciando", update: "Atualizando" }[
      busy ?? "start"
    ],
  };

  return (
    <article
      className="service"
      onContextMenu={(event) => {
        event.preventDefault();
        // Pelo teclado não há posição do mouse: usa a do card.
        const box = event.currentTarget.getBoundingClientRect();
        const keyboard = event.clientX === 0 && event.clientY === 0;
        onMenu(
          keyboard ? box.left + 24 : event.clientX,
          keyboard ? box.top + 24 : event.clientY,
        );
      }}
    >
      <div className="serviceTop">
        <ServiceIcon {...service} />
        <span
          className={`serviceStatus ${state ?? ""}`}
          title={status?.ms != null ? `${status.ms} ms` : undefined}
        >
          <span className={`dot ${state ?? ""}`} />
          {state ? label[state] : "..."}
        </span>
      </div>

      <div className="serviceInfo">
        <h3>
          {/* O link cobre o card inteiro, ver .serviceLink::after */}
          <a
            className="serviceLink"
            href={serviceUrl(service.protocol, service.port)}
            target="_blank"
            rel="noreferrer"
          >
            {service.name}
          </a>
        </h3>
        <button
          className="iconButton editButton"
          onClick={onEdit}
          aria-label={`Editar ${service.name}`}
        >
          {icons.edit}
        </button>
        <button
          className="iconButton moreButton"
          aria-label={`Opções de ${service.name}`}
          aria-haspopup="menu"
          onClick={(event) => {
            const box = event.currentTarget.getBoundingClientRect();
            onMenu(box.right, box.bottom);
          }}
        >
          {icons.more}
        </button>
      </div>
      <p>{service.description}</p>

      <div className="serviceFooter">
        <span>
          {service.protocol === "https" && "https"}:{service.port}
        </span>
        <span style={{ color: categoryColor(service.category) }}>
          {service.category}
        </span>
      </div>
    </article>
  );
}
