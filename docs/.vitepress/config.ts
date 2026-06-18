import { defineConfig } from "vitepress";

export default defineConfig({
  lang: "zh-CN",
  title: "WorldCupIssue — 2026 世界杯桌面盯分伴侣",
  description: "WorldCupIssue 复刻杯况 CupWatch 的 2026 世界杯桌面盯分体验：北京时间赛程、实时比分、积分榜、淘汰赛对阵、托盘、通知与悬浮比分条。",
  cleanUrls: false,
  themeConfig: {
    logo: "/logo.svg",
    siteTitle: "WorldCupIssue",
    nav: [
      { text: "首页", link: "/" },
      { text: "为什么是杯况", link: "/why" },
      { text: "快速上手", link: "/guide/getting-started" },
      { text: "下载", link: "/download" },
      {
        text: "功能",
        items: [
          { text: "今日概览", link: "/features/overview" },
          { text: "赛程", link: "/features/schedule" },
          { text: "比分（LIVE）", link: "/features/scores" },
          { text: "积分榜", link: "/features/standings" },
          { text: "淘汰赛对阵", link: "/features/bracket" },
          { text: "球队与关注", link: "/features/teams" },
          { text: "AI 分析", link: "/features/analysis" },
          { text: "我的预测", link: "/features/predict" }
        ]
      },
      {
        text: "桌面体验",
        items: [
          { text: "盯球铁三角", link: "/guide/desktop-experience" },
          { text: "设置说明", link: "/guide/settings" }
        ]
      },
      { text: "常见问题", link: "/faq" },
      {
        text: "合规与协议",
        items: [
          { text: "合规说明", link: "/legal/compliance" },
          { text: "用户协议与免责声明", link: "/legal/terms" }
        ]
      }
    ],
    sidebar: {
      "/guide/": [
        {
          text: "使用指南",
          items: [
            { text: "快速上手", link: "/guide/getting-started" },
            { text: "盯球铁三角", link: "/guide/desktop-experience" },
            { text: "设置说明", link: "/guide/settings" }
          ]
        }
      ],
      "/features/": [
        {
          text: "功能详解",
          items: [
            { text: "今日概览", link: "/features/overview" },
            { text: "赛程", link: "/features/schedule" },
            { text: "比分（LIVE）", link: "/features/scores" },
            { text: "积分榜", link: "/features/standings" },
            { text: "淘汰赛对阵", link: "/features/bracket" },
            { text: "球队与关注", link: "/features/teams" },
            { text: "AI 分析", link: "/features/analysis" },
            { text: "我的预测", link: "/features/predict" }
          ]
        }
      ],
      "/legal/": [
        {
          text: "合规与协议",
          items: [
            { text: "合规说明", link: "/legal/compliance" },
            { text: "用户协议与免责声明", link: "/legal/terms" }
          ]
        }
      ]
    },
    footer: {
      message: "纯资讯 · 无投注 · 独立第三方工具，与 FIFA 及官方转播机构无关",
      copyright: "© 2026 WorldCupIssue"
    },
    search: {
      provider: "local"
    },
    outline: {
      label: "本页目录",
      level: [2, 3]
    },
    lastUpdated: {
      text: "最后更新"
    }
  }
});
