import type { AppSettings } from "@/types";

export const defaultSettings: AppSettings = {
  theme: "dark",
  launchOnBoot: false,
  notificationsEnabled: true,
  apiBaseUrl: "https://api.example.com",
  apiKey: "",
  activeAiProvider: "gpt",
  aiProfile: {
    provider: "openai_compatible",
    name: "A Book in 30 Minutes",
    baseURL: "http://81.68.73.15:3000/openai/v1",
    model: "gpt-5.5",
    apiKey: "",
    proxyEnabled: false,
    proxyUrl: ""
  },
  geminiProfile: {
    provider: "gemini",
    name: "Gemini",
    baseURL: "https://generativelanguage.googleapis.com/v1beta",
    model: "gemini-flash-latest",
    apiKey: "",
    proxyEnabled: true,
    proxyUrl: "http://127.0.0.1:1080"
  },
  feishuProfile: {
    webhookUrl: "",
    title: "A Book in 30 Minutes",
    testMessage: "听书素材生成工具飞书连通性测试成功。"
  },
  materialProfile: {
    channelName: "半小时听完一本书",
    categoryName: "半小时听完一本书",
    categories: ["半小时听完一本书", "睡前听完一本书", "A Book in 30 Minutes"],
    language: "zh-CN",
    targetMinChars: 7500,
    targetMaxChars: 7800,
    extraDirection: "睡前听书风格，温柔、克制、有陪伴感。旁白目标为 30-35 分钟语音，最佳落在 7500~7800 个中文字；标题和简介服务于 YouTube 中文频道。"
  },
  speechProfile: {
    provider: "azure_microsoft",
    speechKey: "",
    regionKeys: {},
    locale: "zh-CN",
    region: "eastasia",
    voiceName: "zh-CN-YunxiNeural",
    outputFormat: "audio-24khz-160kbitrate-mono-mp3",
    rate: "0%",
    pitch: "+0Hz"
  },
  toolProfile: {
    ffmpegPath: "",
    backgroundMusicMode: "single",
    backgroundMusicPath: "D:\\04_GitHub\\world-cup-issue\\a-book-in-30-minutes\\music\\bf.mp3"
  },
  uiProfile: {
    menuFontFamily: "\"Microsoft YaHei UI\", \"Microsoft YaHei\", \"PingFang SC\", \"Noto Sans SC\", \"Segoe UI\", Arial, sans-serif",
    menuFontSize: 13,
    contentFontFamily: "\"Microsoft YaHei UI\", \"Microsoft YaHei\", \"PingFang SC\", \"Noto Sans SC\", \"Segoe UI\", Arial, sans-serif",
    contentFontSize: 12
  },
  pipelineProfile: {
    imageBackend: "xiaohei-production",
    skipExistingMaterials: true,
    skipExistingText: true,
    skipExistingImages: true,
    skipExistingAudio: true,
    skipExistingSubtitles: true,
    skipExistingVideo: true,
    skipExistingPublish: true
  }
};
