import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { BrowserRouter } from "react-router";
import { isTauri } from "@tauri-apps/api/core";
import App from "./App";
import "./index.css";
import { useUiStore } from "@/stores/ui";

// 启动即应用主题(避免 FOUC)
useUiStore.getState().setTheme(useUiStore.getState().theme);

// 桌面形态禁用 webview 原生右键菜单(刷新/检查/另存为等网页项);
// 输入框 / 文本域 / contentEditable 保留(剪切复制粘贴是编辑刚需)。
if (isTauri()) {
  document.addEventListener("contextmenu", (e) => {
    const el = e.target as HTMLElement | null;
    if (el?.closest("input, textarea, [contenteditable='true']")) return;
    e.preventDefault();
  });
}

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 30_000,
      refetchOnWindowFocus: false,
      retry: 1,
    },
  },
});

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <App />
      </BrowserRouter>
    </QueryClientProvider>
  </StrictMode>,
);
