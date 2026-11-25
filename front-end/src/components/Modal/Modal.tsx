import { useEffect, useRef, type ReactNode } from "react";
import { icons } from "../../lib/icons";
import "./Modal.css";

type Props = {
  title: string;
  subtitle: string;
  icon: ReactNode;
  size?: "wide" | "full";
  onClose: () => void;
  children: ReactNode;
};

/** O dialog nativo já prende o foco e fecha com Esc. */
export default function Modal({
  title,
  subtitle,
  icon,
  size,
  onClose,
  children,
}: Props) {
  const ref = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const dialog = ref.current;
    dialog?.showModal();
    // O autoFocus do React não serve: roda com o dialog ainda fechado.
    dialog?.querySelector<HTMLElement>("[data-autofocus]")?.focus();
    return () => dialog?.close();
  }, []);

  return (
    <dialog
      ref={ref}
      className={`modal ${size ?? ""}`}
      aria-label={title}
      onCancel={(event) => {
        if (event.target !== event.currentTarget) return;
        event.preventDefault();
        onClose();
      }}
    >
      <header className="modalHeader">
        <span className="modalIcon">{icon}</span>
        <div>
          <h2>{title}</h2>
          <p>{subtitle}</p>
        </div>
        <button className="iconButton" onClick={onClose} aria-label="Fechar">
          {icons.close}
        </button>
      </header>
      {children}
    </dialog>
  );
}
