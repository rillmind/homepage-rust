import { categoryColor } from "../../lib/categories";
import { icons } from "../../lib/icons";
import { serviceUrl } from "../../lib/serviceUrl";
import type { Service, Status } from "../../lib/types";
import ServiceIcon from "./ServiceIcon";

type Props = {
  service: Service;
  status?: Status;
  onEdit: () => void;
};

export default function ServiceCard({ service, status, onEdit }: Props) {
  const state = status && (status.online ? "online" : "offline");
  const label = { online: "Online", offline: "Offline" };

  return (
    <article className="service">
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
          className="iconButton"
          onClick={onEdit}
          aria-label={`Editar ${service.name}`}
        >
          {icons.edit}
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
