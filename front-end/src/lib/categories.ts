export const CATEGORIES = [
  "Mídia",
  "Rede & DNS",
  "Automação & IoT",
  "Armazenamento",
  "Ferramentas",
];

const COLORS = ["sky", "red", "peach", "blue", "yellow", "pink", "green", "teal"];

/** Cor fixa por categoria, também para as criadas pelo usuário. */
export function categoryColor(category: string) {
  const known = CATEGORIES.indexOf(category);
  const index =
    known >= 0
      ? known
      : [...category].reduce((sum, char) => sum + char.charCodeAt(0), 0);
  return `var(--ctp-${COLORS[index % COLORS.length]})`;
}
