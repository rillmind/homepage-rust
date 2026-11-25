import { useState, type SubmitEvent } from "react";
import { changePassword, errorText } from "../../lib/api";
import { formatUptime } from "../../lib/format";
import { icons } from "../../lib/icons";
import {
  DEFAULTS,
  INTERVALS,
  THEMES,
  type Settings as Values,
} from "../../lib/settings";
import type { Service, Stats } from "../../lib/types";
import Modal from "../Modal/Modal";
import ServiceForm from "../Services/ServiceForm";
import ServiceIcon from "../Services/ServiceIcon";
import type { ServicesState } from "../Services/useServices";
import "./Settings.css";

const TABS = [
  { id: "general", label: "Geral", icon: icons.settings },
  { id: "network", label: "Rede & IP", icon: icons.network },
  { id: "account", label: "Conta", icon: icons.lock },
  { id: "apps", label: "Aplicativos", icon: icons.apps },
  { id: "about", label: "Sobre", icon: icons.info },
] as const;

type Props = {
  settings: Values;
  onSave: (settings: Values) => void;
  onClose: () => void;
  stats: Stats | null;
  services: ServicesState;
};

export default function Settings({
  settings,
  onSave,
  onClose,
  stats,
  services,
}: Props) {
  const [tab, setTab] = useState<(typeof TABS)[number]["id"]>("general");
  const [draft, setDraft] = useState(settings);
  const [editing, setEditing] = useState<Service | null>(null);
  const list = services.services ?? [];
  const net = stats?.network.find((item) => item.primary);

  const submit = (event: SubmitEvent) => {
    event.preventDefault();
    onSave({ ...draft, title: draft.title.trim() || DEFAULTS.title });
    onClose();
  };

  return (
    <>
      <Modal
        title="Configurações do Servidor"
        subtitle="Preferências da homepage neste navegador"
        icon={icons.settings}
        size="wide"
        onClose={onClose}
      >
        <div className="settings">
          <nav className="settingsTabs">
            {TABS.map(({ id, label, icon }) => (
              <button
                key={id}
                className={id === tab ? "sidebarItem active" : "sidebarItem"}
                onClick={() => setTab(id)}
              >
                <span>{icon}</span> {label}
              </button>
            ))}
          </nav>

          <div className="settingsBody">
            {tab === "general" && (
              <form className="settingsForm" onSubmit={submit}>
                <label className="field">
                  <span>Título da página</span>
                  <input
                    maxLength={40}
                    value={draft.title}
                    onChange={(e) =>
                      setDraft({ ...draft, title: e.target.value })
                    }
                  />
                </label>
                <label className="field">
                  <span>Tema Catppuccin</span>
                  <select
                    value={draft.theme}
                    onChange={(e) =>
                      setDraft({
                        ...draft,
                        theme: e.target.value as Values["theme"],
                      })
                    }
                  >
                    {THEMES.map((theme) => (
                      <option key={theme}>{theme}</option>
                    ))}
                  </select>
                </label>
                <label className="field">
                  <span>Atualização do monitor</span>
                  <select
                    value={draft.interval}
                    onChange={(e) =>
                      setDraft({ ...draft, interval: Number(e.target.value) })
                    }
                  >
                    {INTERVALS.map((seconds) => (
                      <option key={seconds} value={seconds}>
                        A cada {seconds}s
                      </option>
                    ))}
                  </select>
                </label>
                <div className="modalFooter">
                  <button type="button" className="button" onClick={onClose}>
                    Cancelar
                  </button>
                  <button className="button primary">Salvar Alterações</button>
                </div>
              </form>
            )}

            {tab === "network" && (
              <dl>
                <dt>Interface principal</dt>
                <dd>{net?.name ?? "--"}</dd>
                <dt>IP do servidor</dt>
                <dd>{stats?.ip ?? "--"}</dd>
                <dt>Endereço usado agora</dt>
                <dd>{window.location.host}</dd>
              </dl>
            )}

            {tab === "account" && <PasswordForm />}

            {tab === "apps" &&
              (list.length === 0 ? (
                <p className="message">Nenhum serviço cadastrado.</p>
              ) : (
                <ul className="settingsApps">
                  {list.map((item) => (
                    <li key={item.id}>
                      <ServiceIcon {...item} />
                      <span>{item.name}</span>
                      <small>:{item.port}</small>
                      <button
                        className="iconButton"
                        onClick={() => setEditing(item)}
                        aria-label={`Editar ${item.name}`}
                      >
                        {icons.edit}
                      </button>
                    </li>
                  ))}
                </ul>
              ))}

            {tab === "about" && (
              <dl>
                <dt>Projeto</dt>
                <dd>homepage-rust</dd>
                <dt>Servidor</dt>
                <dd>{stats?.hostname ?? "--"}</dd>
                <dt>Uptime</dt>
                <dd>
                  {stats?.uptime != null ? formatUptime(stats.uptime) : "--"}
                </dd>
                <dt>Feito com</dt>
                <dd>Rust, axum, React e Catppuccin</dd>
              </dl>
            )}
          </div>
        </div>
      </Modal>

      {editing && (
        <ServiceForm
          service={editing}
          categories={list.map((item) => item.category)}
          onSave={services.save}
          onRemove={services.remove}
          onClose={() => setEditing(null)}
        />
      )}
    </>
  );
}

function PasswordForm() {
  const [current, setCurrent] = useState("");
  const [next, setNext] = useState("");
  const [message, setMessage] = useState("");

  const submit = async (event: SubmitEvent) => {
    event.preventDefault();
    try {
      await changePassword(current, next);
      setCurrent("");
      setNext("");
      setMessage("Senha trocada. As outras sessões foram encerradas.");
    } catch (error) {
      setMessage(errorText(error));
    }
  };

  return (
    <form className="settingsForm" onSubmit={submit}>
      <label className="field">
        <span>Senha atual</span>
        <input
          type="password"
          required
          autoComplete="current-password"
          value={current}
          onChange={(e) => setCurrent(e.target.value)}
        />
      </label>
      <label className="field">
        <span>Nova senha (mínimo de 6 caracteres)</span>
        <input
          type="password"
          required
          minLength={6}
          maxLength={200}
          autoComplete="new-password"
          value={next}
          onChange={(e) => setNext(e.target.value)}
        />
      </label>
      <p role="status">{message}</p>
      <div className="modalFooter">
        <button className="button primary">Trocar senha</button>
      </div>
    </form>
  );
}
