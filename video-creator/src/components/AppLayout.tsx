import { useEffect, useState, type MouseEvent, type ReactNode } from "react";
import clsx from "clsx";
import { Copy, Minus, Square, Video, X } from "lucide-react";
import { APP_NAME, APP_SUBTITLE } from "@/config/app";
import { navItems, routeMeta } from "@/config/navigation";
import { useAppStore } from "@/store/useAppStore";

export function AppLayout({ children }: { children: ReactNode }) {
  const route = useAppStore((state) => state.route);
  const setRoute = useAppStore((state) => state.setRoute);
  const version = useAppStore((state) => state.version);
  const current = routeMeta[route];
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    let dispose: (() => void) | undefined;
    let canceled = false;

    async function bindWindowState() {
      try {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        const appWindow = getCurrentWindow();
        if (!canceled) setIsMaximized(await appWindow.isMaximized());
        dispose = await appWindow.onResized(async () => setIsMaximized(await appWindow.isMaximized()));
      } catch {
        // Browser preview has no native window API.
      }
    }

    void bindWindowState();
    return () => {
      canceled = true;
      dispose?.();
    };
  }, []);

  return (
    <div className="studio-shell">
      <header className="studio-header" onMouseDown={(event) => void startWindowDrag(event)}>
        <div className="brand-inline">
          <Video size={26} />
          <div>
            <strong>{APP_NAME}</strong>
            <span>{APP_SUBTITLE}</span>
          </div>
          <b>Desktop</b>
          <b>SQLite</b>
          <b>Steps</b>
          <b>Skills</b>
        </div>
        <div className="window-actions">
          <span>v{version}</span>
          <button type="button" title="最小化" onClick={() => void minimizeWindow()}>
            <Minus size={15} />
          </button>
          <button type="button" title={isMaximized ? "还原" : "最大化"} onClick={() => void toggleMaximizeWindow()}>
            {isMaximized ? <Copy size={14} /> : <Square size={13} />}
          </button>
          <button type="button" title="关闭" onClick={() => void closeWindow()}>
            <X size={14} />
          </button>
        </div>
      </header>

      <nav className="studio-tabs">
        {navItems.map((item) => (
          <button className={clsx(item.key === route && "active")} key={item.key} onClick={() => setRoute(item.key)} type="button">
            {item.label}
          </button>
        ))}
      </nav>

      <main className="studio-main">
        <div className="breadcrumb">{current.breadcrumb}</div>
        {children}
      </main>
    </div>
  );
}

async function startWindowDrag(event: MouseEvent<HTMLElement>) {
  if ((event.target as HTMLElement).closest("button, input, select, textarea, a")) return;
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().startDragging();
  } catch {
    // Browser preview has no native window API.
  }
}

async function minimizeWindow() {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().minimize();
  } catch {
    // Browser preview has no native window API.
  }
}

async function toggleMaximizeWindow() {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const appWindow = getCurrentWindow();
    if (await appWindow.isMaximized()) {
      await appWindow.unmaximize();
    } else {
      await appWindow.maximize();
    }
  } catch {
    // Browser preview has no native window API.
  }
}

async function closeWindow() {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().close();
  } catch {
    // Browser preview has no native window API.
  }
}
