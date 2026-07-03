import { Composition } from "remotion";
import { AnimatedBarChart } from "./BarChartVideo";

export function RemotionRoot() {
  return (
    <Composition
      id="AnimatedBarChart"
      component={AnimatedBarChart}
      durationInFrames={150}
      fps={30}
      width={1280}
      height={720}
    />
  );
}
