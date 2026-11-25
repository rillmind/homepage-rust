import { icons } from "../../lib/icons";
import type { Notice } from "../Services/useServices";
import "./Notices.css";

type Props = { notices: Notice[]; onClose: (id: number) => void };

export default function Notices({ notices, onClose }: Props) {
  return (
    <div className="notices" role="status">
      {notices.map((notice) => (
        <div key={notice.id} className={`notice ${notice.kind}`}>
          <span className="noticeIcon">{icons[notice.kind]}</span>
          <div className="noticeBody">
            <p>{notice.message}</p>
            {notice.details && (
              <details>
                <summary>Log do contêiner</summary>
                <pre>{notice.details}</pre>
              </details>
            )}
          </div>
          <button
            className="iconButton"
            aria-label="Fechar aviso"
            onClick={() => onClose(notice.id)}
          >
            {icons.close}
          </button>
        </div>
      ))}
    </div>
  );
}
