/**
 * 客户端 / UI 态(zustand)—— **仅** 主题、移动端侧栏开合、SSE 连接状态(TECH §1.4)。
 * 禁复制服务端数据(那归 react-query)。
 */
import { create } from "zustand";

type Theme = "light" | "dark";
const THEME_KEY = "autohttps-theme";

function readInitialTheme(): Theme {
  const saved = localStorage.getItem(THEME_KEY);
  if (saved === "light" || saved === "dark") return saved;
  return window.matchMedia?.("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

/** 把主题应用到根节点 .dark class(设计 §10-H7)并持久化。 */
export function applyTheme(theme: Theme) {
  document.documentElement.classList.toggle("dark", theme === "dark");
  localStorage.setItem(THEME_KEY, theme);
}

interface UiState {
  theme: Theme;
  setTheme: (t: Theme) => void;
  toggleTheme: () => void;
  mobileNavOpen: boolean;
  setMobileNavOpen: (open: boolean) => void;
  sseConnected: boolean;
  setSseConnected: (c: boolean) => void;
}

export const useUiStore = create<UiState>((set, get) => ({
  theme: readInitialTheme(),
  setTheme: (t) => {
    applyTheme(t);
    set({ theme: t });
  },
  toggleTheme: () => {
    const next: Theme = get().theme === "dark" ? "light" : "dark";
    applyTheme(next);
    set({ theme: next });
  },
  mobileNavOpen: false,
  setMobileNavOpen: (open) => set({ mobileNavOpen: open }),
  sseConnected: false,
  setSseConnected: (c) => set({ sseConnected: c }),
}));
