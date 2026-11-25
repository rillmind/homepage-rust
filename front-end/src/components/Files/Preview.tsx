import { useEffect, useState } from "react";
import { downloadUrl, errorText, readText, saveText } from "../../lib/api";
import { formatBytes } from "../../lib/format";
import type { Found } from "../../lib/types";
import Modal from "../Modal/Modal";
import { iconOf, kindOf } from "./fileTypes";

type Props = { item: Found; readOnly: boolean; onClose: () => void };

export default function Preview({ item, readOnly, onClose }: Props) {
  const [dirty, setDirty] = useState(false);
  const kind = kindOf(item);
  const url = downloadUrl(item.path, true);

  const close = () => {
    if (!dirty || window.confirm("Descartar as alterações?")) onClose();
  };

  return (
    <Modal
      title={item.name}
      subtitle={formatBytes(item.size)}
      icon={iconOf(item)}
      size="full"
      onClose={close}
    >
      <div className="preview">
        {kind === "image" && <img src={url} alt={item.name} />}
        {kind === "video" && <video src={url} controls autoPlay />}
        {kind === "audio" && <audio src={url} controls autoPlay />}
        {kind === "pdf" && <iframe src={url} title={item.name} />}
        {(kind === "text" || kind === "file") && (
          <TextEditor path={item.path} readOnly={readOnly} onDirty={setDirty} />
        )}
        {kind === "archive" && <p>Este tipo não tem visualização.</p>}
      </div>
      <div className="modalFooter previewFooter">
        <a className="button" href={downloadUrl(item.path)}>
          Baixar
        </a>
      </div>
    </Modal>
  );
}

type EditorProps = {
  path: string;
  readOnly: boolean;
  onDirty: (dirty: boolean) => void;
};

function TextEditor({ path, readOnly, onDirty }: EditorProps) {
  const [content, setContent] = useState<string | null>(null);
  const [version, setVersion] = useState("");
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let active = true;
    readText(path).then(
      (text) => {
        if (!active) return;
        setContent(text.content);
        setVersion(text.version);
      },
      (error) => active && setMessage(errorText(error)),
    );
    return () => {
      active = false;
    };
  }, [path]);

  const save = async () => {
    if (content === null || busy) return;
    setBusy(true);
    try {
      const saved = await saveText(path, content, version);
      setVersion(saved.version);
      setMessage("Salvo");
      onDirty(false);
    } catch (error) {
      setMessage(errorText(error));
    }
    setBusy(false);
  };

  if (content === null) return <p>{message || "Carregando..."}</p>;

  return (
    <div className="editor">
      <textarea
        aria-label="Conteúdo"
        spellCheck={false}
        readOnly={readOnly || busy}
        value={content}
        onChange={(event) => {
          setContent(event.target.value);
          setMessage("");
          onDirty(true);
        }}
        onKeyDown={(event) => {
          if ((event.ctrlKey || event.metaKey) && event.key === "s") {
            event.preventDefault();
            if (!readOnly) save();
          }
        }}
      />
      <div className="modalFooter">
        <span role="status">{readOnly ? "Somente leitura" : message}</span>
        {!readOnly && (
          <button className="button primary" onClick={save} disabled={busy}>
            Salvar
          </button>
        )}
      </div>
    </div>
  );
}
