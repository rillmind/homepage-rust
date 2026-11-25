import { useEffect, useState } from "react";

export type View = "overview" | "files" | "network";

/** "#files:/home" -> tela e pasta na URL, para o botão voltar funcionar. */
function parse(): { view: View; path: string } {
  let hash = window.location.hash.slice(1);
  try {
    hash = decodeURIComponent(hash);
  } catch {
    // Endereço digitado errado: usa como veio.
  }
  if (hash === "network") return { view: "network", path: "/" };
  if (hash === "files" || hash.startsWith("files:/")) {
    return { view: "files", path: hash.slice(6) || "/" };
  }
  return { view: "overview", path: "/" };
}

export function goTo(view: View, path?: string) {
  window.location.hash =
    path === undefined
      ? view
      : `${view}:${encodeURI(path).replace(/#/g, "%23")}`;
}

export function useHash() {
  const [state, setState] = useState(parse);
  useEffect(() => {
    const update = () => setState(parse());
    window.addEventListener("hashchange", update);
    return () => window.removeEventListener("hashchange", update);
  }, []);
  return state;
}
