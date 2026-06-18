import { useEffect, useState, type MouseEvent, type ReactNode } from "react";
import {
  Clock3,
  Copy,
  Eye,
  Grid2X2,
  Minus,
  Moon,
  Shield,
  Square,
  Trophy,
  X
} from "lucide-react";
import clsx from "clsx";
import { navGroups, routeMeta } from "@/config/navigation";
import { APP_VERSION } from "@/data/worldCupData";
import { useCupStore } from "@/store/useCupStore";

export function AppLayout({ children }: { children: ReactNode }) {
  const route = useCupStore((state) => state.route);
  const setRoute = useCupStore((state) => state.setRoute);
  const matches = useCupStore((state) => state.matches);
  const settings = useCupStore((state) => state.settings);
  const toggleSpoiler = useCupStore((state) => state.toggleSpoiler);
  const beijingTime = useBeijingTime();
  const [isMaximized, setIsMaximized] = useState(false);
  const current = routeMeta[route];
  const liveCount = matches.filter((match) => match.status === "live").length;

  useEffect(() => {
    let dispose: (() => void) | undefined;
    let canceled = false;

    async function bindWindowState() {
      try {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        const appWindow = getCurrentWindow();
        if (!canceled) setIsMaximized(await appWindow.isMaximized());
        dispose = await appWindow.onResized(async () => {
          setIsMaximized(await appWindow.isMaximized());
        });
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

  const minimizeWindow = async () => {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().minimize();
    } catch {
      // Browser preview has no native window API.
    }
  };

  const toggleMaximizeWindow = async () => {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      const appWindow = getCurrentWindow();
      const maximized = await appWindow.isMaximized();
      if (maximized) {
        await appWindow.unmaximize();
      } else {
        await appWindow.maximize();
      }
      setIsMaximized(!maximized);
    } catch {
      // Browser preview has no native window API.
    }
  };

  const closeWindow = async () => {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().close();
    } catch {
      // Browser preview has no native window API.
    }
  };

  const startWindowDrag = async (event: MouseEvent<HTMLElement>) => {
    if ((event.target as HTMLElement).closest("button, input, select, textarea, a")) return;
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().startDragging();
    } catch {
      // Browser preview has no native window API.
    }
  };

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-icon">
            <Trophy size={24} />
          </div>
          <div>
            <div className="brand-title">WorldCupIssue</div>
            <div className="brand-subtitle">世界杯组手 · 桌面版</div>
          </div>
        </div>

        <nav className="nav">
          {navGroups.map((group) => (
            <div className="nav-group" key={group.title}>
              <div className="nav-title">{group.title}</div>
              {group.items.map((item) => (
                <button
                  key={item.key}
                  className={clsx("nav-item", item.key === route && "active")}
                  onClick={() => setRoute(item.key)}
                  type="button"
                >
                  <span className="nav-icon">{item.icon}</span>
                  <span>{item.label}</span>
                  {item.key === "scores" && liveCount > 0 && <span className="live-nav-badge">{liveCount}</span>}
                  {item.badge && <span className={clsx("mini-badge", item.badge === "AI" ? "gold" : "green")}>{item.badge}</span>}
                </button>
              ))}
            </div>
          ))}
        </nav>

        <div className="compliance">
          <div><Shield size={15} /> 纯资讯 · 无投注</div>
          <p>独立第三方工具，与 FIFA 及官方转播机构无关。v{APP_VERSION}</p>
        </div>
      </aside>

      <main className="main">
        <header className="topbar" onMouseDown={(event) => void startWindowDrag(event)}>
          <div className="page-title-row">
            <button className="square-btn" type="button" aria-label="折叠菜单">
              <Grid2X2 size={18} />
            </button>
            <div>
              <h1>{current.label}</h1>
              <p>{current.breadcrumb}</p>
            </div>
          </div>
          <div className="top-actions">
            <div className="clock-pill">
              <Clock3 className="clock-icon" size={17} />
              <span>{beijingTime}</span>
              <small>北京时间 UTC+8</small>
            </div>
            <button className={clsx("square-btn", settings?.spoilerMode && "active")} type="button" onClick={() => void toggleSpoiler()} title="防剧透">
              <Eye size={18} />
            </button>
            <button className="square-btn ghost" type="button" title="深色模式">
              <Moon size={18} />
            </button>
            <button className="window-btn" type="button" title="最小化" onClick={() => void minimizeWindow()}>
              <Minus size={15} />
            </button>
            <button className="window-btn" type="button" title={isMaximized ? "还原" : "最大化"} onClick={() => void toggleMaximizeWindow()}>
              {isMaximized ? <Copy size={14} /> : <Square size={13} />}
            </button>
            <button className="window-btn" type="button" title="关闭" onClick={() => void closeWindow()}>
              <X size={14} />
            </button>
          </div>
        </header>
        <section className="content">{children}</section>
      </main>
    </div>
  );
}

function useBeijingTime() {
  const [time, setTime] = useState(() => formatBeijingTime());

  useEffect(() => {
    const update = () => setTime(formatBeijingTime());
    update();
    const timer = window.setInterval(update, 1000);
    return () => window.clearInterval(timer);
  }, []);

  return time;
}

function formatBeijingTime() {
  return new Intl.DateTimeFormat("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
    timeZone: "Asia/Shanghai"
  }).format(new Date());
}
