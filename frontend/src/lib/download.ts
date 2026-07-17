/**
 * 导出文件的交付通道分流(方案A / 契约 §5,设计 §H10 形态差异)。
 *
 * 两形态**同一后端导出端点**(`GET .../export`,返回 PEM),仅"落盘方式"不同:
 * - **桌面形态**(`runMode=desktop` 且 Tauri IPC 就绪):走 Tauri 原生**保存对话框**
 *   (`@tauri-apps/plugin-dialog` `save()` 选路径)+ `@tauri-apps/plugin-fs` `writeFile` 写盘——
 *   真正的"选路径"体验;用户取消对话框时返回 `false`(非错误)。
 * - **服务器形态 / 兜底**:浏览器 blob → objectURL → `<a download>` 触发下载(WebView2 亦兼容此路径)。
 *
 * 直接 `fetch` 而非走 `api.get`,因导出是二进制 PEM(非 JSON 包络);失败抛错交调用方 toast。
 * `desktop` 判据来自 `/app-info` 的 runMode(调用方经 `useAppInfo()` 传入)。Tauri 插件用动态 import,
 * 服务器形态不加载其 JS。
 */
import { isTauri } from "@tauri-apps/api/core";
import { API_BASE } from "@/lib/api";

export interface DownloadOptions {
  /** runMode=desktop(来自 /app-info)。桌面下走原生保存对话框,否则浏览器下载。 */
  desktop?: boolean;
}

/**
 * 导出并落盘。返回 `true` 表示已保存/已触发下载;返回 `false` 表示用户取消了原生保存对话框
 * (调用方据此决定是否提示"导出完成")。任何网络/写入错误照常抛出。
 */
export async function downloadFile(
  path: string,
  filename: string,
  opts: DownloadOptions = {},
): Promise<boolean> {
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
  const bytes = new Uint8Array(await res.arrayBuffer());

  // 桌面形态:原生保存对话框选路径 + writeFile 写盘。isTauri() 二次确认 IPC 就绪,
  // 万一 runMode 判为桌面但非 Tauri 环境(不应发生),安全退回 webview 下载。
  if (opts.desktop && isTauri()) {
    const dot = filename.lastIndexOf(".");
    const ext = dot > 0 ? filename.slice(dot + 1) : "";
    const [{ save }, { writeFile }] = await Promise.all([
      import("@tauri-apps/plugin-dialog"),
      import("@tauri-apps/plugin-fs"),
    ]);
    const target = await save({
      defaultPath: filename,
      canCreateDirectories: true,
      filters: ext ? [{ name: `${ext.toUpperCase()} 文件`, extensions: [ext] }] : undefined,
    });
    if (target === null) return false; // 用户取消:非错误
    await writeFile(target, bytes);
    return true;
  }

  // 服务器形态 / 兜底:blob → objectURL → anchor 下载。
  const type = res.headers.get("content-type") ?? "application/octet-stream";
  const url = URL.createObjectURL(new Blob([bytes], { type }));
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(url);
  return true;
}
