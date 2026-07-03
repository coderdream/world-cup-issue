import { Home, Info, Settings, SquareTerminal } from "lucide-react";
import type { RouteKey } from "@/types";

export interface NavItem {
  key: RouteKey;
  label: string;
  icon: React.ReactNode;
}

export const navItems: NavItem[] = [
  { key: "home", label: "首页", icon: <Home size={18} /> },
  { key: "logs", label: "操作日志", icon: <SquareTerminal size={18} /> },
  { key: "settings", label: "配置", icon: <Settings size={18} /> },
  { key: "about", label: "关于", icon: <Info size={18} /> }
];

export const routeMeta: Record<RouteKey, { label: string; breadcrumb: string }> = {
  home: { label: "首页", breadcrumb: "Tauri Framework > 首页" },
  logs: { label: "操作日志", breadcrumb: "Tauri Framework > 操作日志" },
  settings: { label: "配置", breadcrumb: "Tauri Framework > 配置" },
  about: { label: "关于", breadcrumb: "Tauri Framework > 关于" }
};
