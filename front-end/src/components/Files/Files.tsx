import {
  useEffect,
  useRef,
  useState,
  type DragEvent,
  type MouseEvent,
} from "react";
import {
  createItem,
  downloadUrl,
  errorText,
  getJobs,
  listFiles,
  renameItem,
  searchFiles,
  transferItems,
  trashItems,
} from "../../lib/api";
import { normalize } from "../../lib/format";
import { icons } from "../../lib/icons";
import type { Found, Job, Listing } from "../../lib/types";
import { goTo } from "../../lib/useHash";
import Activity from "./Activity";
import FileView from "./FileView";
import NamePrompt from "./NamePrompt";
import Preview from "./Preview";
import Trash from "./Trash";
import { baseName, joinPath, parentOf } from "./fileTypes";
import { useUploads } from "./useUploads";
import "./Files.css";

type Sort = "name" | "modified" | "size";
type Clipboard = { mode: "copy" | "move"; items: string[] };
type Dialog =
  | { type: "folder" | "file" | "trash" }
  | { type: "rename" | "preview"; item: Found };

const SHORTCUTS = ["/", "/home", "/DATA", "/media", "/mnt"];
const COLLATOR = new Intl.Collator("pt-BR", { numeric: true });

function readFlag(key: string) {
  try {
    return localStorage.getItem(key) === "1";
  } catch {
    return false;
  }
}

function writeFlag(key: string, value: boolean) {
  try {
    localStorage.setItem(key, value ? "1" : "0");
  } catch {
    // Sem armazenamento, vale só até fechar a página.
  }
}

function sortItems(items: Found[], sort: Sort) {
  return items.toSorted((a, b) => {
    if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
    const order = sort === "name" ? 0 : b[sort] - a[sort];
    return order || COLLATOR.compare(a.name, b.name);
  });
}

export default function Files({ path }: { path: string }) {
  const [listing, setListing] = useState<Listing | null>(null);
  const [error, setError] = useState("");
  const [reload, setReload] = useState(0);
  const [query, setQuery] = useState("");
  const [search, setSearch] = useState("");
  const [results, setResults] = useState<{
    found: Found[];
    partial: boolean;
  } | null>(null);
  const [sort, setSort] = useState<Sort>("name");
  const [grid, setGrid] = useState(() => readFlag("homepage-files-grid"));
  const [hidden, setHidden] = useState(() => readFlag("homepage-files-hidden"));
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [anchor, setAnchor] = useState("");
  const [clipboard, setClipboard] = useState<Clipboard | null>(null);
  const [dialog, setDialog] = useState<Dialog | null>(null);
  const [dragging, setDragging] = useState(false);
  const [jobs, setJobs] = useState<Job[]>([]);
  const [jobsCleared, setJobsCleared] = useState(0);
  const [jobsStarted, setJobsStarted] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const refresh = () => setReload((n) => n + 1);
  const uploads = useUploads((dir) => {
    if (dir === listing?.path) refresh();
  });

  // Ao mudar de pasta, começa do zero.
  const [shownPath, setShownPath] = useState(path);
  if (shownPath !== path) {
    setShownPath(path);
    setListing(null);
    setSelected(new Set());
    setQuery("");
    setSearch("");
    setResults(null);
    setError("");
  }

  useEffect(() => {
    const abort = new AbortController();
    listFiles(path, abort.signal).then(
      (next) => {
        if (abort.signal.aborted) return;
        setListing(next);
        setError("");
      },
      (err) => {
        if (!abort.signal.aborted) setError(errorText(err));
      },
    );
    return () => abort.abort();
  }, [path, reload]);

  useEffect(() => {
    if (!search) return;
    const abort = new AbortController();
    searchFiles(path, search, abort.signal).then(
      (next) => {
        if (!abort.signal.aborted) setResults(next);
      },
      (err) => {
        if (!abort.signal.aborted) setError(errorText(err));
      },
    );
    return () => abort.abort();
  }, [path, search, reload]);

  // Acompanha os trabalhos só enquanto há algum ativo.
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    let wasActive = jobsStarted > 0;
    const abort = new AbortController();
    const poll = async () => {
      try {
        const list = await getJobs(abort.signal);
        if (abort.signal.aborted) return;
        setJobs(list);
        const active = list.some(
          (job) => job.state === "waiting" || job.state === "running",
        );
        if (active) timer = setTimeout(poll, 700);
        else if (wasActive) setReload((n) => n + 1);
        wasActive = active;
      } catch {
        if (!abort.signal.aborted) timer = setTimeout(poll, 3000);
      }
    };
    poll();
    return () => {
      abort.abort();
      clearTimeout(timer);
    };
  }, [jobsStarted]);

  const dir = listing?.path ?? path;
  const searching = search !== "";
  const readOnly = listing?.readOnly ?? true;
  const filter = normalize(query);
  const source: Found[] = searching
    ? (results?.found ?? [])
    : (listing?.entries ?? []).map((entry) => ({
        ...entry,
        path: joinPath(dir, entry.name),
      }));
  const items = sortItems(
    source.filter(
      (item) =>
        (hidden || !item.name.startsWith(".")) &&
        (searching || normalize(item.name).includes(filter)),
    ),
    sort,
  );
  // Só vale o que está na tela, para não excluir um item filtrado sem ver.
  const chosen = items.filter((item) => selected.has(item.path));
  const chosenPaths = chosen.map((item) => item.path);
  const single = chosen.length === 1 ? chosen[0] : null;

  const attempt = async (action: () => Promise<unknown>) => {
    setError("");
    try {
      await action();
      return true;
    } catch (err) {
      setError(errorText(err));
      return false;
    }
  };

  const open = (item: Found) => {
    if (item.isDir) goTo("files", item.path);
    else setDialog({ type: "preview", item });
  };

  const toggle = (item: Found) => {
    const next = new Set(selected);
    if (!next.delete(item.path)) next.add(item.path);
    setSelected(next);
    setAnchor(item.path);
  };

  const select = (item: Found, event: MouseEvent) => {
    if (event.shiftKey) {
      const from = items.findIndex((other) => other.path === anchor);
      const to = items.indexOf(item);
      const range = items.slice(
        Math.min(Math.max(from, 0), to),
        Math.max(from, to) + 1,
      );
      setSelected(new Set(range.map((other) => other.path)));
    } else if (event.ctrlKey || event.metaKey) {
      toggle(item);
    } else {
      setSelected(new Set([item.path]));
      setAnchor(item.path);
    }
  };

  const copy = (mode: Clipboard["mode"]) => {
    if (chosenPaths.length > 0) setClipboard({ mode, items: chosenPaths });
  };

  const paste = async () => {
    if (!clipboard || readOnly) return;
    const { mode, items: paths } = clipboard;
    const ok = await attempt(() => transferItems(mode, paths, dir));
    if (!ok) return;
    if (mode === "move") setClipboard(null);
    setJobsStarted((n) => n + 1);
  };

  const remove = async () => {
    if (chosenPaths.length === 0) return;
    const what =
      single?.name ?? `${chosenPaths.length.toLocaleString("pt-BR")} itens`;
    if (!window.confirm(`Mover ${what} para a lixeira?`)) return;
    await attempt(() => trashItems(chosenPaths));
    setSelected(new Set());
    refresh();
  };

  const download = () => {
    for (const item of chosen.filter((other) => !other.isDir)) {
      const link = document.createElement("a");
      link.href = downloadUrl(item.path);
      link.download = item.name;
      link.click();
    }
  };

  // Sem lista de dependências: os atalhos usam sempre o estado atual.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement;
      const typing =
        target.matches("textarea, select, input:not([type=checkbox])") ||
        document.querySelector("dialog[open]");
      if (typing) return;
      const combo = event.ctrlKey || event.metaKey;
      const key = event.key.toLowerCase();
      const run = (action: () => unknown) => {
        event.preventDefault();
        action();
      };
      if (combo && key === "a") {
        run(() => setSelected(new Set(items.map((item) => item.path))));
      } else if (combo && key === "c" && !window.getSelection()?.toString()) {
        run(() => copy("copy"));
      } else if (combo && key === "x") run(() => copy("move"));
      else if (combo && key === "v") run(paste);
      else if (key === "delete") run(remove);
      else if (key === "escape") run(() => setSelected(new Set()));
      else if (key === "f2" && single) {
        run(() => setDialog({ type: "rename", item: single }));
      } else if (key === "backspace" && dir !== "/") {
        run(() => goTo("files", parentOf(dir)));
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  const drop = (event: DragEvent) => {
    event.preventDefault();
    setDragging(false);
    if (readOnly) return;
    // Pastas arrastadas não têm conteúdo para enviar.
    const files = [...event.dataTransfer.items]
      .filter((item) => item.webkitGetAsEntry()?.isFile)
      .map((item) => item.getAsFile())
      .filter((file) => file !== null);
    if (files.length > 0) uploads.add(dir, files);
    else setError("Só é possível enviar arquivos, não pastas");
  };

  const crumbs = dir.split("/").filter(Boolean);
  const shownJobs = jobs.filter(
    (job) =>
      job.id > jobsCleared ||
      job.state === "waiting" ||
      job.state === "running",
  );

  return (
    <section
      className={dragging ? "files dragging" : "files"}
      aria-label="Arquivos"
      onDragOver={(event) => {
        event.preventDefault();
        setDragging(true);
      }}
      onDragLeave={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget as Node)) {
          setDragging(false);
        }
      }}
      onDrop={drop}
    >
      <div className="filters">
        {SHORTCUTS.map((shortcut) => (
          <button
            key={shortcut}
            className={shortcut === dir ? "filter active" : "filter"}
            onClick={() => goTo("files", shortcut)}
          >
            {shortcut === "/" ? "Raiz" : shortcut.slice(1)}
          </button>
        ))}
        <button
          className="filter filesTrash"
          onClick={() => setDialog({ type: "trash" })}
        >
          {icons.remove} Lixeira
        </button>
      </div>

      <div className="filesBar">
        <nav className="crumbs" aria-label="Caminho">
          <button onClick={() => goTo("files", "/")} aria-label="Raiz">
            {icons.disk}
          </button>
          {crumbs.map((name, index) => (
            <span key={index}>
              /
              <button
                onClick={() =>
                  goTo("files", "/" + crumbs.slice(0, index + 1).join("/"))
                }
              >
                {name}
              </button>
            </span>
          ))}
          {readOnly && listing && <small>somente leitura</small>}
        </nav>
        <form
          className="search"
          onSubmit={(event) => {
            event.preventDefault();
            setResults(null);
            setSearch(query.trim());
          }}
        >
          <span>{icons.search}</span>
          <input
            type="search"
            aria-label="Filtrar ou buscar"
            placeholder="Filtrar (Enter busca nas subpastas)"
            value={query}
            onChange={(event) => {
              setQuery(event.target.value);
              setSearch("");
              setResults(null);
            }}
          />
        </form>
      </div>

      <div className="filesBar">
        <div className="filesActions">
          <button
            className="button"
            disabled={readOnly}
            onClick={() => inputRef.current?.click()}
          >
            {icons.upload} Enviar
          </button>
          <button
            className="button"
            disabled={readOnly}
            onClick={() => setDialog({ type: "folder" })}
          >
            {icons.newFolder} Nova pasta
          </button>
          <button
            className="button"
            disabled={readOnly}
            onClick={() => setDialog({ type: "file" })}
          >
            {icons.file} Novo arquivo
          </button>
          {clipboard && (
            <button className="button" disabled={readOnly} onClick={paste}>
              {icons.paste} Colar ({clipboard.items.length})
            </button>
          )}
          {chosen.length > 0 && (
            <>
              <button
                className="button"
                disabled={chosen.every((item) => item.isDir)}
                onClick={download}
              >
                {icons.download} Baixar
              </button>
              <button className="button" onClick={() => copy("copy")}>
                {icons.copy} Copiar
              </button>
              <button className="button" onClick={() => copy("move")}>
                {icons.cut} Recortar
              </button>
              {single && (
                <button
                  className="button"
                  onClick={() => setDialog({ type: "rename", item: single })}
                >
                  {icons.edit} Renomear
                </button>
              )}
              <button className="button danger" onClick={remove}>
                {icons.remove} Excluir ({chosen.length})
              </button>
            </>
          )}
        </div>
        <div className="filesActions">
          <select
            className="button"
            aria-label="Ordenar por"
            value={sort}
            onChange={(event) => setSort(event.target.value as Sort)}
          >
            <option value="name">Nome</option>
            <option value="modified">Mais recentes</option>
            <option value="size">Maiores</option>
          </select>
          <button
            className={hidden ? "iconButton on" : "iconButton"}
            aria-pressed={hidden}
            title="Mostrar ocultos"
            onClick={() => {
              writeFlag("homepage-files-hidden", !hidden);
              setHidden(!hidden);
            }}
          >
            {icons.hidden}
          </button>
          <button
            className={grid ? "iconButton on" : "iconButton"}
            aria-pressed={grid}
            title="Ver em grade"
            onClick={() => {
              writeFlag("homepage-files-grid", !grid);
              setGrid(!grid);
            }}
          >
            {icons.apps}
          </button>
        </div>
      </div>

      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      {results?.partial && (
        <p>A busca parou no limite. Seja mais específico.</p>
      )}

      {(searching ? results : listing) ? (
        <FileView
          key={`${dir}|${search}|${grid}`}
          items={items}
          grid={grid}
          selected={selected}
          showParent={searching}
          onSelect={select}
          onToggle={toggle}
          onOpen={open}
        />
      ) : (
        !error && (
          <p className="message">
            {searching ? "Procurando..." : "Carregando..."}
          </p>
        )
      )}

      <Activity
        jobs={shownJobs}
        uploads={uploads.uploads}
        onCancelUpload={uploads.cancel}
        onClear={() => {
          uploads.clear();
          setJobsCleared(Math.max(0, ...jobs.map((job) => job.id)));
        }}
      />

      <input
        ref={inputRef}
        type="file"
        multiple
        hidden
        onChange={(event) => {
          uploads.add(dir, [...(event.target.files ?? [])]);
          event.target.value = "";
        }}
      />

      {(dialog?.type === "folder" || dialog?.type === "file") && (
        <NamePrompt
          title={dialog.type === "folder" ? "Nova pasta" : "Novo arquivo"}
          initial=""
          onSubmit={async (name) => {
            await createItem(dialog.type as "folder" | "file", dir, name);
            refresh();
          }}
          onClose={() => setDialog(null)}
        />
      )}
      {dialog?.type === "rename" && (
        <NamePrompt
          title="Renomear"
          initial={baseName(dialog.item.path)}
          onSubmit={async (name) => {
            await renameItem(dialog.item.path, name);
            setSelected(new Set());
            refresh();
          }}
          onClose={() => setDialog(null)}
        />
      )}
      {dialog?.type === "preview" && (
        <Preview
          item={dialog.item}
          readOnly={readOnly}
          onClose={() => {
            setDialog(null);
            refresh();
          }}
        />
      )}
      {dialog?.type === "trash" && (
        <Trash
          onChange={refresh}
          onPurge={() => setJobsStarted((n) => n + 1)}
          onClose={() => setDialog(null)}
        />
      )}
    </section>
  );
}
