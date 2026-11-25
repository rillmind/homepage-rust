import { icons } from "../../lib/icons";
import type { Entry } from "../../lib/types";

export type Kind =
  | "folder"
  | "image"
  | "video"
  | "audio"
  | "pdf"
  | "archive"
  | "text"
  | "code"
  | "file";

const EXTENSIONS: Partial<Record<Kind, string>> = {
  image: "png jpg jpeg gif webp avif bmp",
  video: "mp4 webm mkv mov m4v ogv",
  audio: "mp3 ogg wav flac m4a opus aac",
  pdf: "pdf",
  archive: "zip tar gz xz zst bz2 7z rar",
  text: "txt md log csv ini conf cfg env",
  code: "json yaml yml toml sh bash zsh py js ts tsx jsx css html xml svg rs go c h cpp java sql service",
};

const KINDS = new Map(
  Object.entries(EXTENSIONS).flatMap(([kind, list]) =>
    list.split(" ").map((extension) => [extension, kind as Kind] as const),
  ),
);

export function kindOf(entry: Entry): Kind {
  if (entry.isDir) return "folder";
  const dot = entry.name.lastIndexOf(".");
  return KINDS.get(entry.name.slice(dot + 1).toLowerCase()) ?? "file";
}

export const hasThumb = (entry: Entry) =>
  !entry.isDir && /\.(png|jpe?g|gif|webp)$/i.test(entry.name);

export const iconOf = (entry: Entry) => icons[kindOf(entry)];

export const joinPath = (dir: string, name: string) =>
  dir === "/" ? "/" + name : `${dir}/${name}`;

export const parentOf = (path: string) =>
  path.slice(0, path.lastIndexOf("/")) || "/";

export const baseName = (path: string) => path.slice(path.lastIndexOf("/") + 1);
