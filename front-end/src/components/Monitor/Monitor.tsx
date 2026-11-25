import { formatBytes, formatNumber, formatRate } from "../../lib/format";
import { icons } from "../../lib/icons";
import type { Stats } from "../../lib/types";
import StatsCard from "./StatsCard";
import "./Monitor.css";

const percentText = (percent?: number) =>
  percent === undefined ? undefined : percent.toFixed(0) + "%";

function tempState(percent: number) {
  if (percent >= 90) return "Crítica";
  return percent >= 75 ? "Alta" : "Normal";
}

export default function Monitor({ stats }: { stats: Stats | null }) {
  const { cpu, memory, disk, temp } = stats ?? {};
  const memoryPercent = memory ? (memory.used / memory.total) * 100 : 0;
  const net = stats?.network.find((item) => item.primary);

  const celsius = temp?.celsius ?? null;
  // Sem limite crítico do sensor, 100 °C enche a barra.
  const tempPercent =
    celsius === null ? 0 : (celsius / (temp?.critical ?? 100)) * 100;

  return (
    <section className="monitor" aria-label="Monitor do sistema">
      <StatsCard
        icon={icons.cpu}
        title="CPU"
        color="mauve"
        header={percentText(cpu?.percent)}
        value={cpu?.mhz ? formatNumber(cpu.mhz / 1000) + " GHz" : undefined}
        details={cpu ? `${cpu.cores} threads` : ""}
        percent={cpu?.percent ?? 0}
      />
      <StatsCard
        icon={icons.memory}
        title="Memória"
        color="blue"
        header={memory ? percentText(memoryPercent) : undefined}
        value={memory ? formatBytes(memory.used) : undefined}
        details={memory ? `de ${formatBytes(memory.total)}` : ""}
        percent={memoryPercent}
      />
      <StatsCard
        icon={icons.disk}
        title="Disco"
        color="teal"
        header={percentText(disk?.percent)}
        value={disk ? formatBytes(disk.used) : undefined}
        details={disk ? `Livre: ${formatBytes(disk.free)}` : ""}
        percent={disk?.percent ?? 0}
      />
      <StatsCard
        icon={icons.temp}
        title="Temperatura"
        color="peach"
        header={celsius === null ? undefined : tempState(tempPercent)}
        value={celsius === null ? undefined : celsius.toFixed(0) + " °C"}
        details={temp?.fan == null ? "" : `Fan: ${temp.fan} RPM`}
        percent={tempPercent}
      />
      <StatsCard
        icon={icons.traffic}
        title="Rede"
        color="sky"
        header={net?.name}
        value={net ? `${icons.down} ${formatRate(net.rx)}` : undefined}
        details={net ? `${icons.up} ${formatRate(net.tx)}` : ""}
        percent={
          net?.speed
            ? ((Math.max(net.rx, net.tx) * 8) / (net.speed * 1e6)) * 100
            : undefined
        }
      />
    </section>
  );
}
