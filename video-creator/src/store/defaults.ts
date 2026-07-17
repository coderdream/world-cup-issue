import type { AppSettings } from "@/types";

export const defaultSettings: AppSettings = {
  theme: "dark",
  launchOnBoot: false,
  notificationsEnabled: true,
  apiBaseUrl: "https://api.example.com",
  apiKey: "",
  javaProjectDir: "D:\\04_GitHub\\video-easy-creator",
  javaRuntimeDir: "D:\\05_Green\\VideoEasyCreator-Portable",
  outputDir: "D:\\14_LearnEnglish\\6MinuteEnglish",
  jianyingDraftDir: "D:\\03_Software\\JianyingPro Drafts\\六分钟英语_2606",
  defaultEpisode: "260625",
  quarkYears: "2014,2015,2016,2017,2018,2019,2020,2021,2022,2023,2024,2025,2026",
  aiProfile: {
    provider: "openai_compatible",
    name: "视频工坊",
    baseURL: "http://81.68.73.15:3000/openai/v1",
    model: "gpt-5.5",
    apiKey: ""
  },
  feishuProfile: {
    webhookUrl: "",
    title: "视频工坊",
    testMessage: "飞书连通性测试成功。"
  }
};
