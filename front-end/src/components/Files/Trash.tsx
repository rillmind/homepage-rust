import { useEffect, useState } from "react";
import { errorText, getTrash, purgeItems, restoreItem } from "../../lib/api";
import { icons } from "../../lib/icons";
import type { Trashed } from "../../lib/types";
import Modal from "../Modal/Modal";

type Props = { onChange: () => void; onPurge: () => void; onClose: () => void };

export default function Trash({ onChange, onPurge, onClose }: Props) {
  const [items, setItems] = useState<Trashed[] | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    let active = true;
    getTrash().then(
      (list) => active && setItems(list.reverse()),
      (err) => active && setError(errorText(err)),
    );
    return () => {
      active = false;
    };
  }, []);

  const restore = async (id: string) => {
    try {
      await restoreItem(id);
      setItems((list) => (list ?? []).filter((item) => item.id !== id));
      onChange();
    } catch (err) {
      setError(errorText(err));
    }
  };

  const purge = async (ids: string[], what: string) => {
    if (!window.confirm(`Excluir ${what} para sempre?`)) return;
    try {
      await purgeItems(ids);
      onPurge();
      onClose();
    } catch (err) {
      setError(errorText(err));
    }
  };

  return (
    <Modal
      title="Lixeira"
      subtitle="Os itens ocupam espaço até serem excluídos daqui"
      icon={icons.remove}
      size="wide"
      onClose={onClose}
    >
      <div className="trash">
        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        {items?.length === 0 && <p>A lixeira está vazia.</p>}
        {items?.map((item) => (
          <div key={item.id} className="trashItem">
            <span>{item.isDir ? icons.folder : icons.file}</span>
            <span className="trashPath" title={item.original}>
              {item.original}
            </span>
            <button className="button" onClick={() => restore(item.id)}>
              Restaurar
            </button>
            <button
              className="iconButton"
              aria-label={`Excluir ${item.original}`}
              onClick={() => purge([item.id], item.original)}
            >
              {icons.remove}
            </button>
          </div>
        ))}
        {items !== null && items.length > 0 && (
          <div className="modalFooter">
            <button
              className="button danger"
              onClick={() =>
                purge(
                  items.map((item) => item.id),
                  "tudo da lixeira",
                )
              }
            >
              Esvaziar lixeira
            </button>
          </div>
        )}
      </div>
    </Modal>
  );
}
