import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath, URL } from "node:url";
import { mkdirSync, writeFileSync } from "node:fs";

// 开发期:前端 dev server(5173)把 `/api`(含 SSE `/api/events`)代理到后端 server(默认 8443)。
// 生产:前端产物内嵌进可执行文件,与 API 同源(ARCHITECTURE §4.3)。
const API_TARGET = process.env.VITE_API_TARGET ?? "http://127.0.0.1:8443";

// build 会清空 `dist/`(含被 git 追踪的 `.gitkeep` 占位)。rust-embed 在编译期要求 `frontend/dist`
// 目录存在(crates/api/src/embed.rs);fresh clone 未跑 npm build 时靠该占位让 `cargo build` 不失败。
// closeBundle 里写回 `.gitkeep`,保证本地 build 后占位不丢、始终随 git 追踪。
function keepDistPlaceholder(): Plugin {
  const distDir = fileURLToPath(new URL("./dist", import.meta.url));
  const gitkeep = fileURLToPath(new URL("./dist/.gitkeep", import.meta.url));
  return {
    name: "keep-dist-placeholder",
    closeBundle() {
      mkdirSync(distDir, { recursive: true });
      writeFileSync(gitkeep, "");
    },
  };
}

export default defineConfig({
  plugins: [react(), tailwindcss(), keepDistPlaceholder()],
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
