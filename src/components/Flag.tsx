import clsx from "clsx";
import { getTeam } from "@/utils/standings";
import type { Team } from "@/types";

interface FlagProps {
  teamId: string;
  size?: "sm" | "md" | "lg";
}

export function Flag({ teamId, size = "md" }: FlagProps) {
  const team = getTeam(teamId);
  return <TeamFlag team={team} size={size} />;
}

export function TeamFlag({ team, size = "md" }: { team: Team; size?: "sm" | "md" | "lg" }) {
  const isSlot = team.id.startsWith("slot-");
  return (
    <span
      aria-label={team.nameZh}
      className={clsx("flag", `flag-${size}`, isSlot ? "flag-slot" : `flag-${team.id}`)}
      title={team.nameZh}
    >
      {isSlot && <span className="flag-slot-text">{team.code.replace("SLOT-", "")}</span>}
    </span>
  );
}

export function TeamName({ teamId }: { teamId: string }) {
  const team = getTeam(teamId);
  return (
    <span className="team-name">
      <Flag teamId={teamId} size="sm" />
      <span>{team.nameZh}</span>
    </span>
  );
}
