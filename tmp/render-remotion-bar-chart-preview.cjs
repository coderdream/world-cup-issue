const fs = require("node:fs");
const path = require("node:path");
const { spawnSync } = require("node:child_process");

const outDir = path.join(__dirname, "remotion-bar-chart-frames");
fs.rmSync(outDir, { force: true, recursive: true });
fs.mkdirSync(outDir, { recursive: true });

const width = 1280;
const height = 720;
const fps = 30;
const seconds = 5;
const totalFrames = fps * seconds;
const data = [
  { label: "Q1", value: 42, color: "#21c55d" },
  { label: "Q2", value: 68, color: "#38bdf8" },
  { label: "Q3", value: 54, color: "#f59e0b" },
  { label: "Q4", value: 86, color: "#f43f5e" },
  { label: "Q5", value: 73, color: "#a78bfa" }
];
const max = Math.max(...data.map((item) => item.value));

const easeOut = (t) => 1 - Math.pow(1 - Math.max(0, Math.min(1, t)), 3);

const svgForFrame = (frame) => {
  const titleOpacity = easeOut(frame / 18);
  const gridOpacity = 0.38 * easeOut((frame - 6) / 24);
  const chart = { left: 88, top: 184, right: 88, bottom: 74 };
  const chartWidth = width - chart.left - chart.right;
  const chartHeight = height - chart.top - chart.bottom;
  const gap = 28;
  const barWidth = (chartWidth - gap * 4) / 5;

  const bars = data
    .map((item, index) => {
      const progress = easeOut((frame - index * 4.8) / 27);
      const barHeight = ((item.value / max) * chartHeight * progress);
      const x = chart.left + index * (barWidth + gap);
      const y = chart.top + chartHeight - barHeight - 48;
      const valueOpacity = easeOut((progress - 0.45) / 0.55);
      return `
        <text x="${x + barWidth / 2}" y="${y - 16}" text-anchor="middle" fill="#e9fff5" font-size="34" font-weight="800" opacity="${valueOpacity.toFixed(3)}">${Math.round(item.value * progress)}</text>
        <defs>
          <linearGradient id="bar-${index}" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stop-color="${item.color}" />
            <stop offset="100%" stop-color="#123529" />
          </linearGradient>
          <filter id="glow-${index}" x="-30%" y="-30%" width="160%" height="170%">
            <feDropShadow dx="0" dy="18" stdDeviation="18" flood-color="${item.color}" flood-opacity="0.33"/>
          </filter>
        </defs>
        <rect x="${x}" y="${y}" width="${barWidth}" height="${Math.max(0, barHeight)}" rx="8" fill="url(#bar-${index})" stroke="rgba(255,255,255,0.22)" filter="url(#glow-${index})" />
        <text x="${x + barWidth / 2}" y="${chart.top + chartHeight - 6}" text-anchor="middle" fill="#b7c9c0" font-size="24" font-weight="700">${item.label}</text>
      `;
    })
    .join("");

  return `<?xml version="1.0" encoding="UTF-8"?>
  <svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">
    <rect width="${width}" height="${height}" fill="#08110f"/>
    <text x="72" y="98" fill="#8ee7bd" font-size="28" font-weight="700" opacity="${titleOpacity.toFixed(3)}">Revenue Pulse</text>
    <text x="72" y="132" fill="#c9d8d1" font-size="18" opacity="${titleOpacity.toFixed(3)}">Animated 5-bar Remotion chart</text>
    ${[0, 0.25, 0.5, 0.75]
      .map((pos) => `<line x1="${chart.left}" y1="${chart.top + chartHeight * pos}" x2="${width - chart.right}" y2="${chart.top + chartHeight * pos}" stroke="#5e756b" stroke-opacity="${gridOpacity.toFixed(3)}"/>`)
      .join("")}
    ${bars}
  </svg>`;
};

for (let frame = 0; frame < totalFrames; frame += 1) {
  fs.writeFileSync(path.join(outDir, `frame-${String(frame).padStart(4, "0")}.svg`), svgForFrame(frame), "utf8");
}

const stillPath = path.join(__dirname, "remotion-animated-bar-chart.svg");
fs.writeFileSync(stillPath, svgForFrame(45), "utf8");

const outputPath = path.join(__dirname, "remotion-animated-bar-chart.mp4");
const ffmpeg = spawnSync(
  "ffmpeg",
  [
    "-y",
    "-framerate",
    String(fps),
    "-i",
    path.join(outDir, "frame-%04d.svg"),
    "-c:v",
    "libx264",
    "-pix_fmt",
    "yuv420p",
    "-movflags",
    "+faststart",
    outputPath
  ],
  { encoding: "utf8" }
);

if (ffmpeg.status !== 0) {
  process.stderr.write(ffmpeg.stderr);
  process.exit(ffmpeg.status ?? 1);
}

console.log(outputPath);
