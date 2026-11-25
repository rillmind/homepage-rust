import { useEffect, useRef, useState, type MouseEvent } from "react";
import { thumbUrl } from "../../lib/api";
import { formatBytes } from "../../lib/format";
import { icons } from "../../lib/icons";
import type { Found } from "../../lib/types";
import { hasThumb, iconOf, kindOf, parentOf } from "./fileTypes";

const ROW_HEIGHT = 48;
const CELL = { width: 148, height: 172 };
const OVERSCAN = 4;

export type SortKey = "name" | "modified" | "size";
export type Sort = { key: SortKey; reverse: boolean };

const COLUMNS: { key: SortKey; label: string }[] = [
  { key: "name", label: "Nome" },
  { key: "modified", label: "Modificado" },
  { key: "size", label: "Tamanho" },
];

type Props = {
  items: Found[];
  grid: boolean;
  selected: Set<string>;
  showParent: boolean;
  sort: Sort;
  onSort: (key: SortKey) => void;
  onSelect: (item: Found, event: MouseEvent) => void;
  onToggle: (item: Found) => void;
  onToggleAll: () => void;
  onOpen: (item: Found) => void;
};

function formatDate(seconds: number) {
  const date = new Date(seconds * 1000);
  const time = date.toLocaleTimeString("pt-BR", { timeStyle: "short" });
  const today = new Date().toDateString() === date.toDateString();
  return today
    ? `Hoje, ${time}`
    : `${date.toLocaleDateString("pt-BR")} ${time}`;
}

/** Desenha só o que está visível, para pastas enormes não pesarem. */
export default function FileView({
  items,
  grid,
  selected,
  showParent,
  sort,
  onSort,
  onSelect,
  onToggle,
  onToggleAll,
  onOpen,
}: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const [box, setBox] = useState({ top: 0, height: 600, width: 800 });

  useEffect(() => {
    const element = ref.current;
    if (!element) return;
    const measure = () =>
      setBox({
        top: element.scrollTop,
        height: element.clientHeight,
        width: element.clientWidth,
      });
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    element.addEventListener("scroll", measure, { passive: true });
    return () => {
      observer.disconnect();
      element.removeEventListener("scroll", measure);
    };
  }, []);

  const columns = grid ? Math.max(1, Math.floor(box.width / CELL.width)) : 1;
  const height = grid ? CELL.height : ROW_HEIGHT;
  const rows = Math.ceil(items.length / columns);
  const first = Math.max(0, Math.floor(box.top / height) - OVERSCAN);
  const last = Math.ceil((box.top + box.height) / height) + OVERSCAN;
  const visible = items.slice(first * columns, last * columns);

  return (
    <div className="fileView" ref={ref}>
      {!grid && items.length > 0 && (
        <div className="fileHead">
          <input
            type="checkbox"
            aria-label="Selecionar tudo"
            checked={items.every((item) => selected.has(item.path))}
            onChange={onToggleAll}
          />
          {COLUMNS.map(({ key, label }) => (
            <button key={key} className={key} onClick={() => onSort(key)}>
              {key === "modified" && showParent ? "Pasta" : label}
              {sort.key === key && (sort.reverse ? icons.down : icons.up)}
            </button>
          ))}
        </div>
      )}

      <div style={{ height: rows * height, position: "relative" }}>
        <div
          className={grid ? "fileGrid" : "fileList"}
          style={{
            transform: `translateY(${first * height}px)`,
            gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
            gridAutoRows: height,
          }}
        >
          {visible.map((item) => (
            <div
              key={item.path}
              className={selected.has(item.path) ? "file selected" : "file"}
              title={item.name}
              onClick={(event) => onSelect(item, event)}
              onDoubleClick={() => onOpen(item)}
            >
              <input
                type="checkbox"
                aria-label={`Selecionar ${item.name}`}
                checked={selected.has(item.path)}
                onClick={(event) => event.stopPropagation()}
                onDoubleClick={(event) => event.stopPropagation()}
                onChange={() => onToggle(item)}
              />
              <span className={`fileIcon kind-${kindOf(item)}`}>
                {grid && hasThumb(item) ? (
                  <Thumb key={item.modified} item={item} />
                ) : (
                  iconOf(item)
                )}
              </span>
              <span className="fileLabel">
                <button
                  className="fileName"
                  onClick={(event) => {
                    // Com Ctrl ou Shift o clique seleciona, como no resto da linha.
                    if (event.ctrlKey || event.metaKey || event.shiftKey) {
                      return;
                    }
                    event.stopPropagation();
                    onOpen(item);
                  }}
                >
                  {item.name}
                </button>
                {item.isLink && icons.link}
              </span>
              <span className="fileMeta">
                {showParent ? parentOf(item.path) : formatDate(item.modified)}
              </span>
              <span className="fileSize">
                {item.isDir ? "Pasta" : formatBytes(item.size)}
              </span>
            </div>
          ))}
        </div>
      </div>

      {items.length === 0 && (
        <div className="fileEmpty">
          {icons.openFolder}
          <strong>Nada por aqui</strong>
          <span>Arraste arquivos para cá ou use o botão Enviar.</span>
        </div>
      )}
    </div>
  );
}

function Thumb({ item }: { item: Found }) {
  const [failed, setFailed] = useState(false);
  if (failed) return iconOf(item);
  return (
    <img
      src={thumbUrl(item.path, item.modified)}
      alt=""
      loading="lazy"
      onError={() => setFailed(true)}
    />
  );
}
