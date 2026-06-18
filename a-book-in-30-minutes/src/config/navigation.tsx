import { BookOpenText, Info, Settings } from "lucide-react";
import type { RouteKey } from "@/types";

export interface NavItem {
  key: RouteKey;
  label: string;
  icon: React.ReactNode;
}

export const navItems: NavItem[] = [
  { key: "home", label: "素材生成", icon: <BookOpenText size={18} /> },
  { key: "settings", label: "配置", icon: <Settings size={18} /> },
  { key: "about", label: "关于", icon: <Info size={18} /> }
];

export const routeMeta: Record<RouteKey, { label: string; breadcrumb: string }> = {
  home: { label: "素材生成", breadcrumb: "A Book in 30 Minutes > 素材生成" },
  settings: { label: "配置", breadcrumb: "A Book in 30 Minutes > 配置" },
  about: { label: "关于", breadcrumb: "A Book in 30 Minutes > 关于" }
};
