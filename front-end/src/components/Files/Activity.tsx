import { cancelJob } from "../../lib/api";
import { formatBytes } from "../../lib/format";
import { icons } from "../../lib/icons";
import type { Job } from "../../lib/types";
import ProgressBar from "../Monitor/ProgressBar";
import type { Upload } from "./useUploads";

type Props = {
  jobs: Job[];
  uploads: Upload[];
  onCancelUpload: (id: number) => void;
  onClear: () => void;
};

const STATES: Record<string, string> = {
  waiting: "Na fila",
  running: "Em andamento",
  done: "Concluído",
  failed: "Falhou",
  cancelled: "Cancelado",
};

/** Envios, cópias e exclusões em andamento ou recém-terminados. */
export default function Activity({
  jobs,
  uploads,
  onCancelUpload,
  onClear,
}: Props) {
  const rows = [
    ...uploads.map((item) => ({
      ...item,
      key: `u${item.id}`,
      label: `Enviar ${item.name}`,
      done: item.sent,
      cancel: () => onCancelUpload(item.id),
    })),
    ...jobs.map((item) => ({
      ...item,
      key: `j${item.id}`,
      cancel: () => void cancelJob(item.id).catch(() => undefined),
    })),
  ];
  if (rows.length === 0) return null;

  return (
    <section className="activity" aria-label="Atividade">
      <header>
        <strong>Atividade</strong>
        <button className="button" onClick={onClear}>
          Limpar
        </button>
      </header>
      {rows.map((row) => {
        const active = row.state === "waiting" || row.state === "running";
        return (
          <div key={row.key} className="activityRow">
            <span className="activityLabel" title={row.label}>
              {row.label}
            </span>
            <small className={row.state === "failed" ? "error" : undefined}>
              {row.error || STATES[row.state]}
              {active &&
                row.total > 0 &&
                ` ${formatBytes(row.done)} de ${formatBytes(row.total)}`}
            </small>
            {active && (
              <button
                className="iconButton"
                aria-label="Cancelar"
                onClick={row.cancel}
              >
                {icons.close}
              </button>
            )}
            {active && row.total > 0 && (
              <ProgressBar
                label={row.label}
                percent={(row.done / row.total) * 100}
                color="var(--ctp-mauve)"
              />
            )}
          </div>
        );
      })}
    </section>
  );
}
