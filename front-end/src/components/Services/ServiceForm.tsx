import { useState, type SubmitEvent } from "react";
import { probePort } from "../../lib/api";
import { CATEGORIES } from "../../lib/categories";
import { icons } from "../../lib/icons";
import { serviceUrl } from "../../lib/serviceUrl";
import type { Service, ServiceInput } from "../../lib/types";
import Modal from "../Modal/Modal";
import ServiceIcon from "./ServiceIcon";
import type { ServicesState } from "./useServices";

type Props = {
  service?: Service;
  categories: string[];
  onSave: ServicesState["save"];
  onRemove: ServicesState["remove"];
  onClose: () => void;
};

const EMPTY = {
  name: "",
  icon: "",
  protocol: "http" as ServiceInput["protocol"],
  category: CATEGORIES[0],
  description: "",
  container: "",
};

export default function ServiceForm({
  service,
  categories,
  onSave,
  onRemove,
  onClose,
}: Props) {
  const [form, setForm] = useState(service ?? EMPTY);
  // Texto, para o campo poder ficar vazio enquanto se digita.
  const [portText, setPortText] = useState(service ? `${service.port}` : "");
  const [probe, setProbe] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [confirming, setConfirming] = useState(false);

  const port = Number(portText);
  const validPort = Number.isInteger(port) && port >= 1 && port <= 65535;
  const set = (change: Partial<typeof EMPTY>) =>
    setForm((current) => ({ ...current, ...change }));

  const run = async (action: () => Promise<void>) => {
    setBusy(true);
    setError("");
    try {
      await action();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Erro inesperado");
      setBusy(false);
    }
  };

  const submit = (event: SubmitEvent) => {
    event.preventDefault();
    const { name, icon, protocol, category, description, container } = form;
    run(() =>
      onSave(
        { name, icon, protocol, port, category, description, container },
        service?.id,
      ),
    );
  };

  const test = async () => {
    setProbe("Testando...");
    try {
      const { online, ms } = await probePort(port);
      setProbe(online ? `Respondeu em ${ms} ms` : "Sem resposta");
    } catch {
      setProbe("Não foi possível testar");
    }
  };

  return (
    <Modal
      title={service ? "Editar Serviço" : "Adicionar Novo Serviço"}
      subtitle="Um contêiner ou serviço que roda neste servidor"
      icon={service ? icons.edit : icons.add}
      onClose={onClose}
    >
      <form className="serviceForm" onSubmit={submit}>
        <label className="field">
          <span>Nome do serviço (obrigatório)</span>
          <input
            data-autofocus
            required
            maxLength={60}
            value={form.name}
            onChange={(e) => set({ name: e.target.value })}
            placeholder="Jellyfin"
          />
        </label>

        <div className="formIcon">
          <ServiceIcon {...form} />
          <label className="field">
            <span>URL do ícone (SVG ou PNG)</span>
            <input
              type="url"
              pattern="https?://.+"
              maxLength={500}
              value={form.icon}
              onChange={(e) => set({ icon: e.target.value })}
              placeholder="https://..."
            />
          </label>
        </div>

        <div className="formRow">
          <label className="field">
            <span>Protocolo</span>
            <select
              value={form.protocol}
              onChange={(e) =>
                set({ protocol: e.target.value as ServiceInput["protocol"] })
              }
            >
              <option value="http">HTTP</option>
              <option value="https">HTTPS</option>
            </select>
          </label>
          <label className="field">
            <span>Porta (obrigatória)</span>
            <input
              type="number"
              required
              min={1}
              max={65535}
              value={portText}
              onChange={(e) => {
                setPortText(e.target.value);
                setProbe("");
              }}
              placeholder="8096"
            />
          </label>
        </div>

        <label className="field">
          <span>Categoria (escolha ou digite uma nova)</span>
          <input
            required
            maxLength={30}
            list="categories"
            value={form.category}
            onChange={(e) => set({ category: e.target.value })}
          />
          <datalist id="categories">
            {[...new Set([...CATEGORIES, ...categories])].map((name) => (
              <option key={name} value={name} />
            ))}
          </datalist>
        </label>

        <label className="field">
          <span>Descrição rápida</span>
          <input
            maxLength={120}
            value={form.description}
            onChange={(e) => set({ description: e.target.value })}
            placeholder="Mídia e streaming local"
          />
        </label>

        <label className="field">
          <span>Contêiner (vazio: encontra pela porta)</span>
          <input
            maxLength={128}
            pattern="[A-Za-z0-9_.\-]*"
            autoCapitalize="none"
            value={form.container}
            onChange={(e) => set({ container: e.target.value })}
            placeholder="jellyfin"
          />
        </label>

        {validPort && (
          <div className="formTarget">
            <span>{serviceUrl(form.protocol, port)}</span>
            <span role="status">{probe}</span>
            <button type="button" className="button" onClick={test}>
              Testar
            </button>
          </div>
        )}

        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}

        <div className="modalFooter">
          {service && (
            <button
              type="button"
              className="button danger formRemove"
              disabled={busy}
              onClick={() =>
                confirming
                  ? run(() => onRemove(service.id))
                  : setConfirming(true)
              }
            >
              {confirming ? "Confirmar exclusão" : "Excluir"}
            </button>
          )}
          <button type="button" className="button" onClick={onClose}>
            Cancelar
          </button>
          <button className="button primary" disabled={busy}>
            {service ? "Salvar" : "Adicionar"}
          </button>
        </div>
      </form>
    </Modal>
  );
}
