import { formatRate } from "../../lib/format";
import { icons } from "../../lib/icons";
import type { Stats } from "../../lib/types";
import "./Network.css";

export default function Network({ stats }: { stats: Stats | null }) {
  if (stats === null) return <p className="message">Sem conexão...</p>;
  if (stats.network.length === 0) {
    return <p className="message">Nenhuma interface encontrada.</p>;
  }

  return (
    <section className="network" aria-label="Interfaces de rede">
      {stats.network.map((item) => (
        <article key={item.name} className="interface">
          <h2>
            <span className={item.up ? "dot online" : "dot"} />
            {item.name}
            {item.primary && <small>principal</small>}
          </h2>
          <dl>
            <dt>IP</dt>
            <dd>{item.ips.join("\n") || "--"}</dd>
            <dt>Link</dt>
            <dd>
              {!item.up
                ? "Desconectado"
                : item.speed
                  ? `${item.speed} Mbps`
                  : "Conectado"}
            </dd>
            <dt>{icons.down} Download</dt>
            <dd>{formatRate(item.rx)}</dd>
            <dt>{icons.up} Upload</dt>
            <dd>{formatRate(item.tx)}</dd>
          </dl>
        </article>
      ))}
    </section>
  );
}
