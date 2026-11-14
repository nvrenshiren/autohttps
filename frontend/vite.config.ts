import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath, URL } from "node:url";

// 开发期:前端 dev server(5173)把 `/api`(含 SSE `/api/events`)代理到后端 server(默认 8443)。
// 生产:前端产物内嵌进可执行文件,与 API 同源(ARCHITECTURE §4.3)。
const API_TARGET = process.env.VITE_API_TARGET ?? "http://127.0.0.1:8443";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  server: {
    port: 5173,
    proxy: {
      "/api": {
        target: API_TARGET,
        changeOrigin: true,
      },
    },
  },
  build: {
    outDir: "dist",
    // 桌面小端基线:产物无需 legacy;chunk 警告阈值放宽。
    chunkSizeWarningLimit: 1200,
  },
});
