import { formatUptime } from "../../lib/format";
import { icons } from "../../lib/icons";
import type { Stats } from "../../lib/types";
import "./Nav.css";

type Props = {
  title: string;
  stats: Stats | null;
  onSearch: () => void;
  onSettings: () => void;
};

export default function Nav({ title, stats, onSearch, onSettings }: Props) {
  const uptime = stats?.uptime;

  return (
    <header className="nav">
      <div className="navSide">
        <div className="navBrand">
          <span className="navLogo">{icons.server}</span>
          <h1>{title}</h1>
        </div>

        <div className="navStatus">
          <span className={stats ? "dot online" : "dot offline"} />
          {stats ? (stats.ip ?? "Sem rede") : "Sem conexão"}
          {uptime != null && (
            <span className="navUptime">Uptime: {formatUptime(uptime)}</span>
          )}
        </div>
      </div>

      <div className="navClock">
        <span>{icons.clock}</span>
        {stats ? `${stats.date} ${stats.time}` : "--/--/---- --:--"}
      </div>

      <div className="navSide navActions">
        <button className="navSearch" onClick={onSearch} aria-label="Pesquisar">
          {icons.search} <span>Quick search</span> <kbd>Ctrl K</kbd>
        </button>

        <button
          className="button navSettings"
          onClick={onSettings}
          aria-label="Configurações"
        >
          {icons.settings} <span>Configurações</span>
        </button>
      </div>
    </header>
  );
}
