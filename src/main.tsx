import React from "react";
import ReactDOM from "react-dom/client";
import { ConfigProvider, theme } from "antd";
import zhCN from "antd/locale/zh_CN";
import App from "./App";
import "./styles/global.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ConfigProvider
      locale={zhCN}
      theme={{
        algorithm: theme.darkAlgorithm,
        token: {
          colorPrimary: "#00d084",
          colorInfo: "#2f81ff",
          colorWarning: "#facc15",
          colorBgBase: "#06110d",
          colorBgContainer: "#13231a",
          colorBorder: "#294136",
          colorText: "#f3fff9",
          colorTextSecondary: "#8eb0a0",
          borderRadius: 8,
          fontFamily:
            "Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif"
        },
        components: {
          Button: {
            controlHeight: 34,
            borderRadius: 8
          },
          Card: {
            colorBgContainer: "#14251b",
            colorBorderSecondary: "#2b4438"
          },
          Input: {
            colorBgContainer: "#102018",
            colorBorder: "#30483d"
          },
          Select: {
            colorBgContainer: "#102018",
            colorBorder: "#30483d"
          }
        }
      }}
    >
      <App />
    </ConfigProvider>
  </React.StrictMode>
);
