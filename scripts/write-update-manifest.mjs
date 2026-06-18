import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { basename, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const version = JSON.parse(readFileSync(join(root, "package.json"), "utf8")).version;
const nsisDir = join(root, "src-tauri", "target", "x86_64-pc-windows-gnu", "release", "bundle", "nsis");
const releaseDir = join(root, "release", version);

const setup = findFile(nsisDir, new RegExp(`WorldCupIssue_${escapeRegExp(version)}_x64-setup\\.exe$`));
const signatureFile = `${setup}.sig`;

if (!existsSync(signatureFile)) {
  throw new Error(`Missing updater signature: ${signatureFile}`);
}

const signature = readFileSync(signatureFile, "utf8").trim();
const setupName = basename(setup);
const repoReleaseBase = `https://github.com/coderdream/world-cup-issue/releases/download/v${version}`;
const now = new Date().toISOString();

mkdirSync(releaseDir, { recursive: true });

const manifest = {
  version,
  notes: `WorldCupIssue（世界杯组手）v${version}：继续收紧导航和小节标题字号，使界面更贴近 CupWatch 参考图；保留简体中文安装器和顶部栏拖拽。`,
  pub_date: now,
  platforms: {
    "windows-x86_64-nsis": {
      signature,
      url: `${repoReleaseBase}/${setupName}`
    },
    "windows-x86_64": {
      signature,
      url: `${repoReleaseBase}/${setupName}`
    }
  }
};

const inventory = {
  version,
  generatedAt: now,
  files: [
    fileRecord(setup, setupName),
    fileRecord(signatureFile, `${setupName}.sig`)
  ],
  updaterEndpoint: "https://github.com/coderdream/world-cup-issue/releases/latest/download/latest.json",
  uploadTargets: [
    "latest.json",
    setupName,
    `${setupName}.sig`
  ]
};

writeFileSync(join(releaseDir, "latest.json"), `${JSON.stringify(manifest, null, 2)}\n`, "utf8");
writeFileSync(join(releaseDir, "release-inventory.json"), `${JSON.stringify(inventory, null, 2)}\n`, "utf8");

console.log(`Wrote ${join(releaseDir, "latest.json")}`);

function findFile(dir, pattern) {
  if (!existsSync(dir)) {
    throw new Error(`Missing directory: ${dir}`);
  }
  const matches = readdirSync(dir)
    .map((name) => join(dir, name))
    .filter((path) => pattern.test(basename(path)));

  if (matches.length === 0) {
    throw new Error(`No file matching ${pattern} in ${dir}`);
  }

  return matches.sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs)[0];
}

function fileRecord(path, name) {
  const bytes = readFileSync(path);
  return {
    name,
    size: bytes.length,
    sha256: createHash("sha256").update(bytes).digest("hex")
  };
}

function escapeRegExp(input) {
  return input.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
