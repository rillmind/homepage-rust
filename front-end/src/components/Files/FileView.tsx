import { useEffect, useRef, useState, type MouseEvent } from "react";
import { thumbUrl } from "../../lib/api";
import { formatBytes } from "../../lib/format";
import { icons } from "../../lib/icons";
import type { Found } from "../../lib/types";
import { hasThumb, iconOf, kindOf, parentOf } from "./fileTypes";

const ROW_HEIGHT = 40;
const CELL = { width: 136, height: 150 };
const OVERSCAN = 4;

type Props = {
  items: Found[];
  grid: boolean;
  selected: Set<string>;
  showParent: boolean;
  onSelect: (item: Found, event: MouseEvent) => void;
  onToggle: (item: Found) => void;
  onOpen: (item: Found) => void;
};

/** Desenha só o que está visível, para pastas enormes não pesarem. */
export default function FileView({
  items,
  grid,
  selected,
  showParent,
  onSelect,
  onToggle,
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
              <button
                className="fileName"
                onClick={(event) => {
                  // Com Ctrl ou Shift o clique seleciona, como no resto da linha.
                  if (event.ctrlKey || event.metaKey || event.shiftKey) return;
                  event.stopPropagation();
                  onOpen(item);
                }}
              >
                {item.name}
                {item.isLink && <small> {icons.link}</small>}
              </button>
              {!grid && (
                <>
                  <span className="fileMeta">
                    {showParent
                      ? parentOf(item.path)
                      : new Date(item.modified * 1000).toLocaleString("pt-BR", {
                          dateStyle: "short",
                          timeStyle: "short",
                        })}
                  </span>
                  <span className="fileSize">
                    {item.isDir ? "" : formatBytes(item.size)}
                  </span>
                </>
              )}
            </div>
          ))}
        </div>
      </div>
      {items.length === 0 && <p className="message">Nada por aqui.</p>}
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
