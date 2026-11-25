type Props = { label: string; percent: number; color: string };

export default function ProgressBar({ label, percent, color }: Props) {
  const value = Math.min(100, Math.max(0, percent));

  return (
    <div
      className="progress"
      role="progressbar"
      aria-label={label}
      aria-valuenow={Math.round(value)}
      aria-valuemin={0}
      aria-valuemax={100}
    >
      <div
        className="progressFill"
        style={{ width: `${value}%`, backgroundColor: color }}
      />
    </div>
  );
}
