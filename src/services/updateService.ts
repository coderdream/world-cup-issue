import { cupwatchApi } from "@/lib/api/cupwatch";
import { APP_VERSION } from "@/data/worldCupData";
import type { UpdateInfo } from "@/types";

export type UpdateCheckStatus = "latest" | "available" | "installed" | "manual" | "error";

export interface UpdateCheckResult {
  status: UpdateCheckStatus;
  currentVersion: string;
  latestVersion: string;
  message: string;
}

export async function checkAndInstallUpdate(): Promise<UpdateCheckResult> {
  if (isTauriRuntime()) {
    try {
      const { check } = await import("@tauri-apps/plugin-updater");
      const update = await check({ timeout: 20000 });

      if (!update) {
        return {
          status: "latest",
          currentVersion: APP_VERSION,
          latestVersion: APP_VERSION,
          message: "当前已是最新版本。"
        };
      }

      await update.downloadAndInstall();

      try {
        const { relaunch } = await import("@tauri-apps/plugin-process");
        await relaunch();
      } catch {
        return {
          status: "installed",
          currentVersion: update.currentVersion,
          latestVersion: update.version,
          message: `已安装 v${update.version}，请手动重启应用完成升级。`
        };
      }

      return {
        status: "installed",
        currentVersion: update.currentVersion,
        latestVersion: update.version,
        message: `已安装 v${update.version}，正在重启应用。`
      };
    } catch (error) {
      return fallbackUpdateResult(error);
    }
  }

  const mock = await cupwatchApi.checkUpdateMock();
  return fromMockUpdateInfo(mock);
}

function isTauriRuntime() {
  return typeof window !== "undefined" && Boolean(window.__TAURI_INTERNALS__);
}

async function fallbackUpdateResult(error: unknown): Promise<UpdateCheckResult> {
  const mock = await cupwatchApi.checkUpdateMock();
  const fallback = fromMockUpdateInfo(mock);
  const reason = error instanceof Error ? error.message : String(error);

  return {
    ...fallback,
    status: "manual",
    message: `${fallback.message} 自动升级端点暂不可用：${reason}`
  };
}

function fromMockUpdateInfo(info: UpdateInfo): UpdateCheckResult {
  return {
    status: info.available ? "available" : "latest",
    currentVersion: info.currentVersion,
    latestVersion: info.latestVersion,
    message: info.notes
  };
}
