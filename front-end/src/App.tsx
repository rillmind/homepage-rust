import { useEffect, useState } from "react";
import Monitor from "./components/Monitor/Monitor";
import Nav from "./components/Nav/Nav";
import Network from "./components/Network/Network";
import Services from "./components/Services/Services";
import { useServices } from "./components/Services/useServices";
import Settings from "./components/Settings/Settings";
import Sidebar, { type View } from "./components/Sidebar/Sidebar";
import { getStats } from "./lib/api";
import { loadSettings, storeSettings } from "./lib/settings";
import { usePolling } from "./lib/usePolling";
import "./App.css";

export default function App() {
  const [settings, setSettings] = useState(loadSettings);
  const [view, setView] = useState<View>("overview");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [searchRequest, setSearchRequest] = useState(0);
  const stats = usePolling(getStats, settings.interval * 1000);
  const services = useServices();

  useEffect(() => {
    document.documentElement.className = settings.theme;
    document.title = settings.title;
    storeSettings(settings);
  }, [settings]);

  const openSearch = () => {
    setView("overview");
    setSearchRequest((n) => n + 1);
  };

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const combo = (event.ctrlKey || event.metaKey) && event.key === "k";
      if (!combo || document.querySelector("dialog[open]")) return;
      event.preventDefault();
      setView("overview");
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
      />
      <Sidebar selected={view} onSelect={setView} stats={stats} />
      <main>
        {view === "overview" ? (
          <>
            <Monitor stats={stats} />
            <Services {...services} searchRequest={searchRequest} />
          </>
        ) : (
          <Network stats={stats} />
        )}
      </main>
      {settingsOpen && (
        <Settings
          settings={settings}
          onSave={setSettings}
          onClose={() => setSettingsOpen(false)}
          stats={stats}
          services={services}
        />
      )}
    </div>
  );
}
