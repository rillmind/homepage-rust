import { lazy, Suspense, useEffect, useState } from "react";
import Monitor from "./components/Monitor/Monitor";
import Nav from "./components/Nav/Nav";
import Network from "./components/Network/Network";
import Notices from "./components/Notices/Notices";
import Services from "./components/Services/Services";
import { useServices } from "./components/Services/useServices";
import Settings from "./components/Settings/Settings";
import Sidebar from "./components/Sidebar/Sidebar";
import { getStats } from "./lib/api";
import type { Settings as Values } from "./lib/settings";
import { goTo, useHash } from "./lib/useHash";
import { usePolling } from "./lib/usePolling";
import "./Home.css";

// Só baixa o gerenciador quando ele é aberto.
const Files = lazy(() => import("./components/Files/Files"));

type Props = {
  settings: Values;
  onSettings: (settings: Values) => void;
  username: string;
  onLogout: () => void;
};

export default function Home({
  settings,
  onSettings,
  username,
  onLogout,
}: Props) {
  const { view, path } = useHash();
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [searchRequest, setSearchRequest] = useState(0);
  const stats = usePolling(getStats, settings.interval * 1000);
  const services = useServices();

  const openSearch = () => {
    goTo("overview");
    setSearchRequest((n) => n + 1);
  };

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const combo = (event.ctrlKey || event.metaKey) && event.key === "k";
      if (!combo || document.querySelector("dialog[open]")) return;
      event.preventDefault();
      goTo("overview");
      setSearchRequest((n) => n + 1);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <div className="app">
      <Nav
        title={settings.title}
        stats={stats}
        onSearch={openSearch}
        onSettings={() => setSettingsOpen(true)}
        onLogout={onLogout}
        username={username}
      />
      <Sidebar selected={view} stats={stats} />
      <main>
        {view === "overview" && (
          <>
            <Monitor stats={stats} />
            <Services {...services} searchRequest={searchRequest} />
          </>
        )}
        {view === "network" && <Network stats={stats} />}
        {view === "files" && (
          <Suspense fallback={<p className="message">Carregando...</p>}>
            <Files path={path} />
          </Suspense>
        )}
      </main>
      <Notices notices={services.notices} onClose={services.dismiss} />
      {settingsOpen && (
        <Settings
          settings={settings}
          onSave={onSettings}
          onClose={() => setSettingsOpen(false)}
          stats={stats}
          services={services}
        />
      )}
    </div>
  );
}
