import { GitBranch, Info, ListChecks, Settings, SquareTerminal } from "lucide-react";
import type { RouteKey } from "@/types";

export interface NavItem {
  key: RouteKey;
  label: string;
  icon: React.ReactNode;
}

export const navItems: NavItem[] = [
  { key: "home", label: "流水线", icon: <GitBranch size={18} /> },
  { key: "audio", label: "步骤跟踪", icon: <ListChecks size={18} /> },
  { key: "logs", label: "操作日志", icon: <SquareTerminal size={18} /> },
  { key: "settings", label: "配置", icon: <Settings size={18} /> },
  { key: "about", label: "关于", icon: <Info size={18} /> }
];

export const routeMeta: Record<RouteKey, { label: string; breadcrumb: string }> = {
  home: { label: "流水线", breadcrumb: "A Book in 30 Minutes > 流水线" },
  audio: { label: "步骤跟踪", breadcrumb: "A Book in 30 Minutes > 步骤跟踪" },
  logs: { label: "操作日志", breadcrumb: "A Book in 30 Minutes > 操作日志" },
  settings: { label: "配置", breadcrumb: "A Book in 30 Minutes > 配置" },
  about: { label: "关于", breadcrumb: "A Book in 30 Minutes > 关于" }
};
