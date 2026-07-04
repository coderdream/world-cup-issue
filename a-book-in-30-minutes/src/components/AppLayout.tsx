import { useEffect, useMemo, useState, type CSSProperties, type MouseEvent, type ReactNode } from "react";
import clsx from "clsx";
import { BookOpenText, Clock3, Copy, Minus, Moon, PanelLeft, Shield, Square, X } from "lucide-react";
import { APP_NAME, APP_SUBTITLE } from "@/config/app";
import { navItems, routeMeta } from "@/config/navigation";
import { useAppStore } from "@/store/useAppStore";

export function AppLayout({ children }: { children: ReactNode }) {
  const route = useAppStore((state) => state.route);
  const setRoute = useAppStore((state) => state.setRoute);
  const version = useAppStore((state) => state.version);
  const settings = useAppStore((state) => state.settings);
  const current = routeMeta[route];
  const time = useBeijingTime();
  const activeAiProfile = settings.activeAiProvider === "gemini" ? settings.geminiProfile : settings.aiProfile;
  const modelName = activeAiProfile.model.trim() || "未配置模型";
  const [isMaximized, setIsMaximized] = useState(false);
  const fontStyle = useMemo(
    () =>
      ({
        "--menu-font-family": settings.uiProfile.menuFontFamily,
        "--menu-font-size": `${clampFontSize(settings.uiProfile.menuFontSize, 10, 18)}px`,
        "--content-font-family": settings.uiProfile.contentFontFamily,
        "--content-font-size": `${clampFontSize(settings.uiProfile.contentFontSize, 10, 18)}px`
      }) as CSSProperties,
    [settings.uiProfile]
  );

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
    <div className="app-shell" style={fontStyle}>
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-icon">
            <BookOpenText size={22} />
          </div>
          <div>
            <div className="brand-title">{APP_NAME}</div>
            <div className="brand-subtitle">{APP_SUBTITLE}</div>
          </div>
        </div>

        <nav className="nav">
          {navItems.map((item) => (
            <button className={clsx("nav-item", item.key === route && "active")} key={item.key} onClick={() => setRoute(item.key)} type="button">
              <span className="nav-icon">{item.icon}</span>
              <span>{item.label}</span>
            </button>
          ))}
        </nav>

        <div className="compliance">
          <div><Shield size={15} /> 素材工作台</div>
          <p>EPUB 到标题、简介、标签、旁白稿和字幕。v{version}</p>
        </div>
      </aside>

      <main className="main">
        <header className="topbar" onMouseDown={(event) => void startWindowDrag(event)}>
          <div className="page-title-row">
            <button className="square-btn" type="button" aria-label="折叠菜单">
              <PanelLeft size={18} />
            </button>
            <div>
              <h1>{current.label}</h1>
              <p>{current.breadcrumb}</p>
            </div>
          </div>
          <div className="top-actions">
            <div className="model-pill" title={`当前模型：${modelName}`}>
              【{modelName}】
            </div>
            <div className="clock-pill">
              <Clock3 className="clock-icon" size={17} />
              <span>{time}</span>
              <small>北京时间 UTC+8</small>
            </div>
            <button className="square-btn ghost" type="button" title={`主题：${settings.theme}`}>
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

function clampFontSize(value: number, min: number, max: number) {
  if (!Number.isFinite(value)) return min;
  return Math.max(min, Math.min(max, Math.round(value)));
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

function useBeijingTime() {
  const [time, setTime] = useState(() => formatBeijingTime());

  useEffect(() => {
    const timer = window.setInterval(() => setTime(formatBeijingTime()), 1000);
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
