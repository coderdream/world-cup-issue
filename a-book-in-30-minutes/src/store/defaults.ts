import type { AppSettings } from "@/types";

export const defaultSettings: AppSettings = {
  theme: "dark",
  launchOnBoot: false,
  notificationsEnabled: true,
  apiBaseUrl: "https://api.example.com",
  apiKey: "",
  aiProfile: {
    provider: "openai_compatible",
    name: "A Book in 30 Minutes",
    baseURL: "http://81.68.73.15:3000/openai/v1",
    model: "gpt-5.5",
    apiKey: ""
  }
};
