import type { AppSettings } from "@/types";

export const defaultSettings: AppSettings = {
  theme: "dark",
  launchOnBoot: false,
  notificationsEnabled: true,
  apiBaseUrl: "https://api.example.com",
  apiKey: "",
  aiProfile: {
    provider: "openai_compatible",
    name: "Tauri Framework",
    baseURL: "http://81.68.73.15:3000/openai/v1",
    model: "gpt-5.5",
    apiKey: ""
  },
  feishuProfile: {
    webhookUrl: "",
    title: "Tauri Framework",
    testMessage: "飞书连通性测试成功。"
  }
};
