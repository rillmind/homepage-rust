import { useState } from "react";
import { categoryColor } from "../../lib/categories";

type Props = { icon: string; name: string; category: string };

/** Sem ícone, ou se a imagem falhar, mostra a inicial do nome. */
export default function ServiceIcon({ icon, name, category }: Props) {
  const [failed, setFailed] = useState("");

  return (
    <span className="serviceIcon" style={{ color: categoryColor(category) }}>
      {icon && failed !== icon ? (
        <img
          src={icon}
          alt=""
          referrerPolicy="no-referrer"
          onError={() => setFailed(icon)}
        />
      ) : (
        name.trim().charAt(0).toUpperCase() || "?"
      )}
    </span>
  );
}
