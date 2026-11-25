import { useEffect, useState } from "react";
import Auth from "./components/Auth/Auth";
import Home from "./Home";
import { getAuth, logout, SESSION_LOST } from "./lib/api";
import { loadSettings, storeSettings } from "./lib/settings";
import type { AuthStatus } from "./lib/types";

const RETRY_INTERVAL = 3000;

export default function App() {
  const [settings, setSettings] = useState(loadSettings);
  const [auth, setAuth] = useState<AuthStatus | null>(null);
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    document.documentElement.className = settings.theme;
    document.title = settings.title;
    // Pinta a barra do app instalado e do navegador com a cor do tema.
    document
      .querySelector('meta[name="theme-color"]')
      ?.setAttribute(
        "content",
        getComputedStyle(document.documentElement)
          .getPropertyValue("--ctp-mantle")
          .trim(),
      );
    storeSettings(settings);
  }, [settings]);

  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    let active = true;
    const load = () =>
      getAuth().then(
        (status) => active && setAuth(status),
        () => {
          if (active) timer = setTimeout(load, RETRY_INTERVAL);
        },
      );
    load();
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [attempt]);

  useEffect(() => {
    const onLost = () =>
      setAuth((status) => status && { ...status, authenticated: false });
    window.addEventListener(SESSION_LOST, onLost);
    return () => window.removeEventListener(SESSION_LOST, onLost);
  }, []);

  if (auth === null) return <p className="message">Conectando...</p>;

  if (!auth.authenticated) {
    return (
      <Auth
        title={settings.title}
        username={auth.username}
        onDone={() => setAttempt((n) => n + 1)}
      />
    );
  }

  const leave = async () => {
    await logout().catch(() => undefined);
    setAttempt((n) => n + 1);
  };

  return (
    <Home
      settings={settings}
      onSettings={setSettings}
      username={auth.username ?? ""}
      onLogout={leave}
    />
  );
}
