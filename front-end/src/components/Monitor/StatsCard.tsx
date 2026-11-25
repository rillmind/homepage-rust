import type { CSSProperties, ReactNode } from "react";
import ProgressBar from "./ProgressBar";

type Props = {
  icon: ReactNode;
  title: string;
  color: string;
  header?: string;
  value?: ReactNode;
  details?: ReactNode;
  percent?: number;
};

/** Campo sem dado aparece como "--". */
export default function StatsCard({
  icon,
  title,
  color,
  header = "--",
  value = "--",
  details = "",
  percent,
}: Props) {
  const accent = `var(--ctp-${color})`;

  return (
    <article className="card" style={{ "--accent": accent } as CSSProperties}>
      <div className="cardRow cardHeader">
        <h2>
          <span className="cardIcon">{icon}</span> {title}
        </h2>
        <span className="cardAccent">{header}</span>
      </div>
      <div className="cardRow">
        <strong>{value}</strong>
        <small>{details}</small>
      </div>
      {percent !== undefined && (
        <ProgressBar label={title} percent={percent} color={accent} />
      )}
    </article>
  );
}
