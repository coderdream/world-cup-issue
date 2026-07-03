import {
  AbsoluteFill,
  Easing,
  interpolate,
  spring,
  useCurrentFrame,
  useVideoConfig
} from "remotion";

const chartData = [
  { label: "Q1", value: 42, color: "#21c55d" },
  { label: "Q2", value: 68, color: "#38bdf8" },
  { label: "Q3", value: 54, color: "#f59e0b" },
  { label: "Q4", value: 86, color: "#f43f5e" },
  { label: "Q5", value: 73, color: "#a78bfa" }
];

const maxValue = Math.max(...chartData.map((item) => item.value));
const staggerSeconds = 0.16;

export function AnimatedBarChart() {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  const titleOpacity = interpolate(frame, [0, 0.6 * fps], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
    easing: Easing.bezier(0.16, 1, 0.3, 1)
  });

  const gridOpacity = interpolate(frame, [0.2 * fps, 1 * fps], [0, 0.38], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp"
  });

  return (
    <AbsoluteFill
      style={{
        background: "#08110f",
        color: "#f5fff9",
        fontFamily:
          "Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif",
        padding: 72
      }}
    >
      <div style={{ opacity: titleOpacity }}>
        <div
          style={{
            color: "#8ee7bd",
            fontSize: 28,
            fontWeight: 700,
            letterSpacing: 0,
            marginBottom: 10
          }}
        >
          Revenue Pulse
        </div>
        <div style={{ color: "#c9d8d1", fontSize: 18 }}>
          Animated 5-bar Remotion chart
        </div>
      </div>

      <div
        style={{
          bottom: 74,
          display: "grid",
          gap: 28,
          gridTemplateColumns: `repeat(${chartData.length}, 1fr)`,
          left: 88,
          position: "absolute",
          right: 88,
          top: 184
        }}
      >
        {[0, 1, 2, 3].map((line) => (
          <div
            key={line}
            style={{
              borderTop: "1px solid #5e756b",
              left: 0,
              opacity: gridOpacity,
              position: "absolute",
              right: 0,
              top: `${line * 25}%`
            }}
          />
        ))}

        {chartData.map((item, index) => {
          const progress = spring({
            frame,
            fps,
            delay: index * staggerSeconds * fps,
            config: {
              damping: 18,
              mass: 0.85,
              stiffness: 110
            }
          });
          const heightPercent = (item.value / maxValue) * 100 * progress;
          const valueOpacity = interpolate(progress, [0.45, 1], [0, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp"
          });

          return (
            <div
              key={item.label}
              style={{
                alignItems: "center",
                display: "flex",
                flexDirection: "column",
                justifyContent: "flex-end",
                minWidth: 0,
                position: "relative"
              }}
            >
              <div
                style={{
                  color: "#e9fff5",
                  fontSize: 34,
                  fontWeight: 800,
                  marginBottom: 16,
                  opacity: valueOpacity
                }}
              >
                {Math.round(item.value * progress)}
              </div>
              <div
                style={{
                  background: `linear-gradient(180deg, ${item.color}, #123529)`,
                  border: "1px solid rgba(255,255,255,0.22)",
                  borderRadius: 8,
                  boxShadow: `0 18px 48px ${item.color}55`,
                  height: `${heightPercent}%`,
                  minHeight: progress > 0 ? 8 : 0,
                  width: "100%"
                }}
              />
              <div
                style={{
                  color: "#b7c9c0",
                  fontSize: 24,
                  fontWeight: 700,
                  marginTop: 18
                }}
              >
                {item.label}
              </div>
            </div>
          );
        })}
      </div>
    </AbsoluteFill>
  );
}
