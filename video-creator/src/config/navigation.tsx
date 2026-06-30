import { FolderSync, History, Home, Info, ListChecks, Settings, SlidersHorizontal, SquareTerminal } from "lucide-react";
import type { RouteKey } from "@/types";

export interface NavItem {
  key: RouteKey;
  label: string;
  icon: React.ReactNode;
  group: "business" | "system";
}

export const navItems: NavItem[] = [
  { key: "home", label: "执行中心", icon: <Home size={18} />, group: "business" },
  { key: "steps", label: "步骤跟踪", icon: <ListChecks size={18} />, group: "business" },
  { key: "logs", label: "执行日志", icon: <SquareTerminal size={18} />, group: "business" },
  { key: "history", label: "历史记录", icon: <History size={18} />, group: "business" },
  { key: "quark", label: "Quark 同步", icon: <FolderSync size={18} />, group: "business" },
  { key: "skills", label: "Skills", icon: <SlidersHorizontal size={18} />, group: "business" },
  { key: "settings", label: "配置", icon: <Settings size={18} />, group: "system" },
  { key: "about", label: "关于", icon: <Info size={18} />, group: "system" }
];

export const routeMeta: Record<RouteKey, { label: string; breadcrumb: string }> = {
  home: { label: "执行中心", breadcrumb: "视频工坊 > 执行中心" },
  steps: { label: "步骤跟踪", breadcrumb: "视频工坊 > 步骤跟踪" },
  logs: { label: "执行日志", breadcrumb: "视频工坊 > 执行日志" },
  history: { label: "历史记录", breadcrumb: "视频工坊 > 历史记录" },
  quark: { label: "Quark 同步", breadcrumb: "视频工坊 > Quark 同步" },
  skills: { label: "Skills", breadcrumb: "视频工坊 > Skills" },
  settings: { label: "配置", breadcrumb: "视频工坊 > 配置" },
  about: { label: "关于", breadcrumb: "视频工坊 > 关于" }
};
