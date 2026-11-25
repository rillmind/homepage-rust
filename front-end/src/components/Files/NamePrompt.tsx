import { useState, type SubmitEvent } from "react";
import { errorText } from "../../lib/api";
import { icons } from "../../lib/icons";
import Modal from "../Modal/Modal";

type Props = {
  title: string;
  initial: string;
  onSubmit: (name: string) => Promise<void>;
  onClose: () => void;
};

export default function NamePrompt({
  title,
  initial,
  onSubmit,
  onClose,
}: Props) {
  const [name, setName] = useState(initial);
  const [error, setError] = useState("");

  const submit = async (event: SubmitEvent) => {
    event.preventDefault();
    try {
      await onSubmit(name);
      onClose();
    } catch (err) {
      setError(errorText(err));
    }
  };

  return (
    <Modal title={title} subtitle="" icon={icons.edit} onClose={onClose}>
      <form className="serviceForm" onSubmit={submit}>
        <label className="field">
          <span>Nome</span>
          <input
            data-autofocus
            required
            value={name}
            onFocus={(event) => event.target.select()}
            onChange={(event) => setName(event.target.value)}
          />
        </label>
        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        <div className="modalFooter">
          <button type="button" className="button" onClick={onClose}>
            Cancelar
          </button>
          <button className="button primary">Salvar</button>
        </div>
      </form>
    </Modal>
  );
}
