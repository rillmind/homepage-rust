import type { ReactNode } from "react";
import { icons } from "../../lib/icons";
import type { Stats } from "../../lib/types";
import { goTo, type View } from "../../lib/useHash";
import "./Sidebar.css";

const ITEMS: { view: View; label: string; icon: ReactNode }[] = [
  { view: "overview", label: "Overview", icon: icons.overview },
  { view: "files", label: "Arquivos", icon: icons.folders },
  { view: "network", label: "Network", icon: icons.network },
];

type Props = {
  selected: View;
  stats: Stats | null;
};

export default function Sidebar({ selected, stats }: Props) {
  return (
    <nav className="sidebar">
      {ITEMS.map(({ view, label, icon }) => (
        <button
          key={view}
          className={view === selected ? "sidebarItem active" : "sidebarItem"}
          aria-current={view === selected ? "page" : undefined}
          onClick={() => goTo(view)}
        >
          <span>{icon}</span> {label}
        </button>
      ))}

      <div className="sidebarNode">
        <span className={stats ? "dot online" : "dot offline"} />
        <span className="sidebarHost">{stats?.hostname ?? "Servidor"}</span>
        <span className="sidebarCpu">
          {stats?.cpu ? `${stats.cpu.percent.toFixed(0)}% CPU` : "--"}
        </span>
      </div>
    </nav>
  );
}
