/**
 * 触发文件下载(设计 §H10 形态差异为页面级显隐):服务器形态=浏览器下载,桌面 Tauri webview 亦走
 * 此基线通道(原生保存为后续里程碑)。直接 fetch blob → objectURL → anchor,避免 SPA fallback 拦截
 * `/api` 路径;失败抛错交由调用方 toast。
 */
import { API_BASE } from "@/lib/api";

export async function downloadFile(path: string, filename: string): Promise<void> {
  const res = await fetch(API_BASE + path);
  if (!res.ok) {
    let message = res.statusText;
    try {
      const data = (await res.json()) as { error?: { message?: string } };
      message = data?.error?.message ?? message;
    } catch {
      // 非 JSON 错误体:保留 statusText
    }
    throw new Error(message);
  }
  const blob = await res.blob();
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(url);
}
