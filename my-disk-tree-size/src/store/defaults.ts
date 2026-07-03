import type { AppSettings } from "@/types";

export const defaultSettings: AppSettings = {
  theme: "dark",
  defaultPath: "Z:\\",
  sizeUnit: "auto",
  includeHidden: true,
  maxDepth: 8,
  videoRoot: "Z:\\Video",
  excludedDirNames: [
    "node_modules",
    ".pnpm-store",
    ".git",
    ".cache",
    "target",
    "dist",
    "mysql-test",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".venv",
    "venv",
    "bower_components",
    ".gradle",
    ".idea",
    ".vscode",
    "@eaDir",
    "#recycle",
    "照片",
    "BTPanel",
    "btpanel_data",
    "external_data_147",
    "external_data_148",
    "PicAcg",
    "mt",
    "Apple",
    "docker_data"
  ]
};
