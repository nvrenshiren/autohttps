/**
 * 全局 SSE 订阅(common/events.md §5)—— 收到事件 → invalidate 对应 react-query key(事件是失效信号,
 * 非数据源)。里程碑1:后端为心跳骨架、暂无事件;本 hook 建连 + 维护连接状态,实现期即生效。
 * EventSource 自动重连;onopen 重连后主动全量重取兜底。
 */
import { useEffect } from "react";
import { useQueryClient, type QueryClient } from "@tanstack/react-query";
import { API_BASE } from "@/lib/api";
import { qk } from "@/lib/queries";
import { useUiStore } from "@/stores/ui";
import type { EventType, ServerEvent } from "@/bindings";

const EVENT_TYPES: EventType[] = [
  "certificate_status_changed",
  "task_status_changed",
  "task_log_appended",
  "challenge_status_changed",
  "acme_account_status_changed",
  "root_ca_status_changed",
  "dashboard_changed",
];

function handleEvent(qc: QueryClient, raw: string) {
  let ev: ServerEvent;
  try {
    ev = JSON.parse(raw) as ServerEvent;
  } catch {
    return;
  }
  // 红点/聚合几乎总受影响
  qc.invalidateQueries({ queryKey: qk.dashboard });
  switch (ev.type) {
    case "certificate_status_changed":
      qc.invalidateQueries({ queryKey: qk.certificates });
      qc.invalidateQueries({ queryKey: qk.domains });
      break;
    case "task_status_changed":
    case "task_log_appended":
      qc.invalidateQueries({ queryKey: qk.tasks });
      qc.invalidateQueries({ queryKey: qk.certificates });
      break;
    default:
      break;
  }
}

export function useServerEvents() {
  const qc = useQueryClient();
  const setSseConnected = useUiStore((s) => s.setSseConnected);

  useEffect(() => {
    let es: EventSource;
    try {
      es = new EventSource(`${API_BASE}/events`);
    } catch {
      setSseConnected(false);
      return;
    }

    es.onopen = () => {
      setSseConnected(true);
      // 重连兜底:主动全量重取关键列表(common/events §5)
      qc.invalidateQueries({ queryKey: qk.dashboard });
    };
    es.onerror = () => setSseConnected(false);
    for (const t of EVENT_TYPES) {
      es.addEventListener(t, (e) => handleEvent(qc, (e as MessageEvent).data));
    }

    return () => {
      es.close();
      setSseConnected(false);
    };
  }, [qc, setSseConnected]);
}
