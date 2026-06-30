import clsx from "clsx";
import { Switch as AntSwitch } from "antd";
import type { ReactNode } from "react";

export function Panel({ children, className }: { children: ReactNode; className?: string }) {
  return <section className={clsx("panel", className)}>{children}</section>;
}

export function SectionTitle({ icon, title, action, inline = false }: { icon: ReactNode; title: string; action?: ReactNode; inline?: boolean }) {
  return (
    <div className={clsx("section-title", inline && "inline")}>
      <div>{icon}<h2>{title}</h2></div>
      {action && <span>{action}</span>}
    </div>
  );
}

export function Switch({ checked, onChange }: { checked: boolean; onChange: (checked: boolean) => void }) {
  return <AntSwitch checked={checked} onChange={onChange} />;
}
