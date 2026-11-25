import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";
import { errorText, getContainer } from "../../lib/api";
import { icons } from "../../lib/icons";
import { serviceUrl } from "../../lib/serviceUrl";
import type { Container, ContainerAction, Service } from "../../lib/types";

export type MenuTarget = { service: Service; x: number; y: number };

type Props = {
  target: MenuTarget;
  busy: boolean;
  onRun: (action: ContainerAction) => void;
  onEdit: () => void;
  onClose: () => void;
};

const MARGIN = 8;

/** Menu do botão direito. Oferece iniciar ou reiniciar conforme o contêiner. */
export default function ServiceMenu({
  target: { service, x, y },
  busy,
  onRun,
  onEdit,
  onClose,
}: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const [container, setContainer] = useState<Container | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    const abort = new AbortController();
    getContainer(service.id, abort.signal).then(
      (found) => {
        if (!abort.signal.aborted) setContainer(found);
      },
      (err) => {
        if (!abort.signal.aborted) setError(errorText(err));
      },
    );
    return () => abort.abort();
  }, [service.id]);

  // Mantém o menu dentro da tela, também quando o conteúdo muda de tamanho.
  useLayoutEffect(() => {
    const menu = ref.current;
    if (!menu) return;
    const left = Math.min(x, window.innerWidth - menu.offsetWidth - MARGIN);
    const top = Math.min(y, window.innerHeight - menu.offsetHeight - MARGIN);
    menu.style.left = `${Math.max(MARGIN, left)}px`;
    menu.style.top = `${Math.max(MARGIN, top)}px`;
  }, [x, y, container, error]);

  useEffect(() => {
    const menu = ref.current;
    const previous = document.activeElement;
    menu?.querySelector<HTMLElement>("[role=menuitem]")?.focus();

    const outside = (event: Event) => {
      if (!menu?.contains(event.target as Node)) onClose();
    };
    document.addEventListener("pointerdown", outside);
    window.addEventListener("scroll", onClose, { passive: true });
    window.addEventListener("resize", onClose);
    return () => {
      document.removeEventListener("pointerdown", outside);
      window.removeEventListener("scroll", onClose);
      window.removeEventListener("resize", onClose);
      if (previous instanceof HTMLElement) previous.focus();
    };
    // Os eventos valem para a vida do menu.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === "Escape" || event.key === "Tab") {
      event.preventDefault();
      onClose();
      return;
    }
    const step = { ArrowDown: 1, ArrowUp: -1 }[event.key];
    if (!step) return;
    event.preventDefault();
    const items = [
      ...event.currentTarget.querySelectorAll<HTMLElement>(
        "[role=menuitem]:not(:disabled)",
      ),
    ];
    const index = items.indexOf(document.activeElement as HTMLElement);
    items[(index + step + items.length) % items.length]?.focus();
  };

  const pick = (action: () => void) => () => {
    onClose();
    action();
  };

  return (
    <div
      ref={ref}
      className="menu"
      role="menu"
      aria-label={`Opções de ${service.name}`}
      onKeyDown={onKeyDown}
      onContextMenu={(event) => event.preventDefault()}
    >
      <header>
        {service.name}
        {container && container.name !== service.name && ` · ${container.name}`}
      </header>
      <a
        role="menuitem"
        href={serviceUrl(service.protocol, service.port)}
        target="_blank"
        rel="noreferrer"
        onClick={onClose}
      >
        {icons.open} Abrir
      </a>
      {!container && !error && <p>Verificando o contêiner...</p>}
      {error && <p className="error">{error}</p>}
      {container && (
        <button
          role="menuitem"
          disabled={busy}
          onClick={pick(() => onRun(container.running ? "restart" : "start"))}
        >
          {container.running ? icons.restart : icons.start}{" "}
          {container.running ? "Reiniciar serviço" : "Iniciar serviço"}
        </button>
      )}
      {container?.compose && (
        <button
          role="menuitem"
          disabled={busy}
          onClick={pick(() => onRun("update"))}
        >
          {icons.update} Atualizar serviço
        </button>
      )}
      <button role="menuitem" onClick={pick(onEdit)}>
        {icons.edit} Editar
      </button>
    </div>
  );
}
