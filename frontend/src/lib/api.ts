/**
 * 单一 HTTP 客户端层(ARCHITECTURE §3 方案A:前端只经 HTTP/SSE)。
 * baseURL `/api`(REST/SSE 挂此前缀,与 SPA client-side 路由隔离);错误按 `{error:{code,message}}`
 * 包络解析为 `ApiError`(common §4),前端按 `code` 分支处置。
 */
import type { ApiErrorEnvelope, ErrorCode } from "@/bindings";

export const API_BASE = "/api";

export class ApiError extends Error {
  readonly status: number;
  readonly code: ErrorCode;
  readonly details?: unknown;
  constructor(status: number, code: ErrorCode, message: string, details?: unknown) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.code = code;
    this.details = details;
  }
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  const hasBody = body !== undefined;
  const res = await fetch(API_BASE + path, {
    method,
    headers: hasBody ? { "content-type": "application/json" } : undefined,
    body: hasBody ? JSON.stringify(body) : undefined,
  });

  if (res.status === 204) return undefined as T;

  const text = await res.text();
  const data: unknown = text ? JSON.parse(text) : undefined;

  if (!res.ok) {
    const env = data as ApiErrorEnvelope | undefined;
    const err = env?.error;
    throw new ApiError(
      res.status,
      (err?.code as ErrorCode) ?? "internal_error",
      err?.message ?? res.statusText,
      err?.details,
    );
  }
  return data as T;
}

export const api = {
  get: <T>(path: string) => request<T>("GET", path),
  post: <T>(path: string, body?: unknown) => request<T>("POST", path, body),
  patch: <T>(path: string, body?: unknown) => request<T>("PATCH", path, body),
  put: <T>(path: string, body?: unknown) => request<T>("PUT", path, body),
  del: <T = void>(path: string) => request<T>("DELETE", path),
};

/** 拼查询串(忽略 undefined/null/空)。 */
export function qs(params: Record<string, string | number | undefined | null>): string {
  const sp = new URLSearchParams();
  for (const [k, v] of Object.entries(params)) {
    if (v !== undefined && v !== null && v !== "") sp.set(k, String(v));
  }
  const s = sp.toString();
  return s ? `?${s}` : "";
}
