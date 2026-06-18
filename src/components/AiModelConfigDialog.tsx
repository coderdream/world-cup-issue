import { useEffect, useMemo, useState } from "react";
import clsx from "clsx";
import { AlertCircle, CheckCircle2, ClipboardPaste, Copy, KeyRound, PlugZap, Sparkles, X } from "lucide-react";
import { cupwatchApi } from "@/lib/api/cupwatch";
import { buildAiProfileShare, getAiModelConfig, parseAiProfileShare, stringifyAiProfileShare, aiProviderPresets } from "@/domain/aiConfig";
import type { AppSettings, ConnectivityTestResult } from "@/types";

interface AiModelConfigDialogProps {
  open: boolean;
  settings: AppSettings;
  onClose: () => void;
  onSave: (settings: Partial<AppSettings>) => Promise<void>;
}

export function AiModelConfigDialog({ open, settings, onClose, onSave }: AiModelConfigDialogProps) {
  const [draft, setDraft] = useState(() => getAiModelConfig(settings));
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<ConnectivityTestResult | null>(null);
  const [message, setMessage] = useState("");

  useEffect(() => {
    if (!open) return;
    setDraft(getAiModelConfig(settings));
    setStatus(null);
    setMessage("");
  }, [open, settings]);

  const shareText = useMemo(() => JSON.stringify(buildAiProfileShare(draft), null, 2), [draft]);

  if (!open) return null;

  const updateField = (key: keyof typeof draft, value: string) => {
    setDraft((current) => ({ ...current, [key]: value }));
  };

  const preset = aiProviderPresets.find((item) => item.value === draft.provider) ?? aiProviderPresets[0];

  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={onClose}>
      <div
        className="modal-shell ai-config-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="ai-config-title"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <header className="modal-head">
          <div className="modal-title">
            <KeyRound size={16} />
            <h3 id="ai-config-title">配置 AI 模型</h3>
          </div>
          <button className="icon-close-btn" type="button" onClick={onClose} aria-label="关闭">
            <X size={16} />
          </button>
        </header>

        <div className="modal-body">
          <div className="notice green compact">
            <Sparkles size={16} />
            <span>API Key 仅保存在本机；测试连接会向你配置的接口地址发起一次正常请求。</span>
          </div>

          <div className="config-toolbar">
            <button className="outline-btn small" type="button" onClick={() => void pasteShare()}>
              <ClipboardPaste size={14} /> 粘贴配置自动填充
            </button>
            <button className="outline-btn small" type="button" onClick={() => void copyShare()}>
              <Copy size={14} /> 复制当前配置分享
            </button>
          </div>

          <div className="config-grid">
            <label className="field">
              <span>服务商 / 预设</span>
              <select
                value={draft.provider}
                onChange={(event) => {
                  const nextProvider = event.target.value;
                  const nextPreset = aiProviderPresets.find((item) => item.value === nextProvider) ?? preset;
                  setDraft((current) => ({
                    ...current,
                    provider: nextProvider,
                    baseUrl: nextPreset.baseUrl,
                    model: nextPreset.model
                  }));
                }}
              >
                {aiProviderPresets.map((item) => (
                  <option key={item.value} value={item.value}>
                    {item.label}
                  </option>
                ))}
              </select>
            </label>

            <label className="field">
              <span>接口地址 baseURL</span>
              <input value={draft.baseUrl} onChange={(event) => updateField("baseUrl", event.target.value)} />
            </label>

            <div className="split-grid">
              <label className="field">
                <span>模型名</span>
                <input value={draft.model} onChange={(event) => updateField("model", event.target.value)} />
              </label>
              <label className="field">
                <span>配置名称</span>
                <input value={draft.name} onChange={(event) => updateField("name", event.target.value)} />
              </label>
            </div>

            <label className="field">
              <span>API Key</span>
              <input type="password" value={draft.apiKey} onChange={(event) => updateField("apiKey", event.target.value)} />
            </label>

            <div className="config-actions-line">
              <button className="primary-btn small" type="button" onClick={() => void testConnection()} disabled={busy}>
                <PlugZap size={14} /> {busy ? "测试中" : "测试连接"}
              </button>
              <span className={clsx("status-chip", status?.ok ? "success" : status === null ? "" : "error")}>
                {status?.ok ? <CheckCircle2 size={14} /> : status === null ? <AlertCircle size={14} /> : <AlertCircle size={14} />}
                {status?.message ?? "尚未测试"}
              </span>
            </div>

            {message && <p className={clsx("setting-status", message.includes("失败") && "warning")}>{message}</p>}

            <label className="field share-preview">
              <span>当前配置分享</span>
              <textarea readOnly value={shareText} />
            </label>
          </div>
        </div>

        <footer className="modal-foot">
          <button className="danger-btn" type="button" onClick={() => void clearConfig()}>
            清空
          </button>
          <div className="modal-foot-spacer" />
          <button className="outline-btn" type="button" onClick={onClose}>
            取消
          </button>
          <button className="primary-btn" type="button" onClick={() => void saveConfig()} disabled={busy}>
            保存
          </button>
        </footer>
      </div>
    </div>
  );

  async function copyShare() {
    const text = stringifyAiProfileShare(draft);
    await copyText(text);
    setMessage("已复制当前配置分享");
  }

  async function pasteShare() {
    const text = await readText();
    if (!text) {
      setMessage("未读取到配置内容");
      return;
    }
    const patch = parseAiProfileShare(text);
    if (!patch) {
      setMessage("配置格式不正确");
      return;
    }
    setDraft(getAiModelConfig(patch));
    setMessage("已粘贴并填充配置");
  }

  async function testConnection() {
    if (busy) return;
    setBusy(true);
    setMessage("");
    try {
      const result = await cupwatchApi.testAiModelConfig(draft);
      setStatus(result);
      setMessage(result.message);
    } finally {
      setBusy(false);
    }
  }

  async function saveConfig() {
    if (busy) return;
    setBusy(true);
    setMessage("");
    try {
      await onSave({
        aiProvider: draft.provider,
        aiApiKey: draft.apiKey,
        aiBaseUrl: draft.baseUrl,
        aiModel: draft.model,
        aiProfileName: draft.name
      });
      setMessage("已保存当前 AI 配置");
      onClose();
    } finally {
      setBusy(false);
    }
  }

  async function clearConfig() {
    if (busy) return;
    setDraft(getAiModelConfig({}));
    setStatus(null);
    setMessage("已清空并恢复默认配置");
  }

  async function copyText(text: string) {
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(text);
        return;
      }
    } catch {
      // fall through to prompt
    }
    window.prompt("复制当前配置分享", text);
  }

  async function readText() {
    try {
      if (navigator.clipboard?.readText) {
        return await navigator.clipboard.readText();
      }
    } catch {
      // fall through to prompt
    }
    return window.prompt("粘贴 AI 配置分享 JSON") ?? "";
  }
}
