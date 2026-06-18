import type { ReactNode } from "react";
import {
  BarChart3,
  BellRing,
  CalendarDays,
  Grid2X2,
  Info,
  ListOrdered,
  Radio,
  Settings,
  Shield,
  Sparkles
} from "lucide-react";
import type { RouteKey } from "@/types";

export interface RouteMeta {
  key: RouteKey;
  label: string;
  breadcrumb: string;
  icon: ReactNode;
  badge?: "AI" | "娱乐";
}

export interface NavGroup {
  title: string;
  items: RouteMeta[];
}

export const routeMeta: Record<RouteKey, RouteMeta> = {
  overview: { key: "overview", label: "今日概览", breadcrumb: "WorldCupIssue 〉 今日概览", icon: <Grid2X2 /> },
  schedule: { key: "schedule", label: "赛程", breadcrumb: "WorldCupIssue 〉 赛程", icon: <CalendarDays /> },
  scores: { key: "scores", label: "比分", breadcrumb: "WorldCupIssue 〉 比分", icon: <Radio /> },
  standings: { key: "standings", label: "积分榜", breadcrumb: "WorldCupIssue 〉 积分榜", icon: <ListOrdered /> },
  bracket: { key: "bracket", label: "淘汰赛", breadcrumb: "WorldCupIssue 〉 淘汰赛", icon: <BarChart3 /> },
  ai: { key: "ai", label: "AI 分析", breadcrumb: "WorldCupIssue 〉 AI 分析", icon: <Sparkles />, badge: "AI" },
  predictions: { key: "predictions", label: "我的预测", breadcrumb: "WorldCupIssue 〉 我的预测", icon: <BellRing />, badge: "娱乐" },
  teams: { key: "teams", label: "球队", breadcrumb: "WorldCupIssue 〉 球队", icon: <Shield /> },
  settings: { key: "settings", label: "设置", breadcrumb: "WorldCupIssue 〉 设置", icon: <Settings /> },
  about: { key: "about", label: "关于", breadcrumb: "WorldCupIssue 〉 关于", icon: <Info /> }
};

export const navGroups: NavGroup[] = [
  {
    title: "赛事",
    items: [
      routeMeta.overview,
      routeMeta.schedule,
      routeMeta.scores,
      routeMeta.standings,
      routeMeta.bracket,
      routeMeta.ai,
      routeMeta.predictions,
      routeMeta.teams
    ]
  },
  {
    title: "系统",
    items: [routeMeta.settings, routeMeta.about]
  }
];
