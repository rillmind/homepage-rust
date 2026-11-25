export const THEMES = ["mocha", "macchiato", "frappe", "latte"] as const;
export const INTERVALS = [1, 2, 5, 10] as const;

export type Settings = {
  title: string;
  theme: (typeof THEMES)[number];
  interval: number;
};

const KEY = "homepage-settings";
export const DEFAULTS: Settings = {
  title: "Server Homepage",
  theme: "mocha",
  interval: 1,
};

export function loadSettings(): Settings {
  try {
    const saved = JSON.parse(localStorage.getItem(KEY) ?? "{}");
    return {
      title:
        typeof saved.title === "string" && saved.title
          ? saved.title
          : DEFAULTS.title,
      theme: THEMES.includes(saved.theme) ? saved.theme : DEFAULTS.theme,
      interval: INTERVALS.includes(saved.interval)
        ? saved.interval
        : DEFAULTS.interval,
    };
  } catch {
    return DEFAULTS;
  }
}

export function storeSettings(settings: Settings) {
  try {
    localStorage.setItem(KEY, JSON.stringify(settings));
  } catch {
    // Sem armazenamento, a preferência vale só até fechar a página.
  }
}
