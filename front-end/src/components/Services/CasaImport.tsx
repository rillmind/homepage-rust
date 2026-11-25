import { useEffect, useState } from "react";
import { errorText, getCasaApps } from "../../lib/api";
import { icons } from "../../lib/icons";
import type { CasaApp, Service } from "../../lib/types";
import Modal from "../Modal/Modal";
import ServiceIcon from "./ServiceIcon";
import type { ServicesState } from "./useServices";

const CATEGORY = "CasaOS";

type Props = {
  services: Service[];
  onSave: ServicesState["save"];
  onClose: () => void;
};

/** Apps com uma porta já cadastrada não aparecem. */
export default function CasaImport({ services, onSave, onClose }: Props) {
  const [apps, setApps] = useState<CasaApp[] | null>(null);
  const [skipped, setSkipped] = useState<number[]>([]);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let active = true;
    getCasaApps().then(
      (list) => active && setApps(list),
      (err) => active && setError(errorText(err)),
    );
    return () => {
      active = false;
    };
  }, []);

  const available = (apps ?? []).filter(
    (app) => !services.some((item) => item.port === app.port),
  );
  const chosen = available.filter((app) => !skipped.includes(app.port));

  const submit = async () => {
    setBusy(true);
    try {
      for (const app of chosen) {
        const icon = /^https?:\/\//.test(app.icon) ? app.icon : "";
        const name = app.name.slice(0, 60);
        await onSave({ ...app, name, icon, category: CATEGORY, container: "" });
      }
      onClose();
    } catch (err) {
      setError(errorText(err));
      setBusy(false);
    }
  };

  return (
    <Modal
      title="Importar do CasaOS"
      subtitle="Apps instalados que ainda não estão na homepage"
      icon={icons.import}
      onClose={onClose}
    >
      <div className="importList">
        {apps === null && !error && <p>Procurando...</p>}
        {apps !== null && available.length === 0 && (
          <p>Nenhum app novo com porta encontrado.</p>
        )}
        {available.map((app) => (
          <label key={app.port}>
            <input
              type="checkbox"
              checked={!skipped.includes(app.port)}
              onChange={() =>
                setSkipped((list) =>
                  list.includes(app.port)
                    ? list.filter((port) => port !== app.port)
                    : [...list, app.port],
                )
              }
            />
            <ServiceIcon {...app} category={CATEGORY} />
            {app.name}
            <small>:{app.port}</small>
          </label>
        ))}
        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        <div className="modalFooter">
          <button className="button" onClick={onClose}>
            Cancelar
          </button>
          <button
            className="button primary"
            disabled={busy || chosen.length === 0}
            onClick={submit}
          >
            Importar ({chosen.length})
          </button>
        </div>
      </div>
    </Modal>
  );
}
