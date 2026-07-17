/**
 * 全局 SSE 订阅(common/events.md §5)—— 收到事件 → invalidate 对应 react-query key(事件是失效信号,
 * **非数据源**:据 payload 里的标识失效对应列表/详情,由 react-query 重取权威 REST 数据)。
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

/** 取 payload 中的字符串标识(payload 为极简 JSON:仅标识 + 判别字段)。 */
function idOf(p: Record<string, unknown>, key: string): string | undefined {
  const v = p[key];
  return typeof v === "string" ? v : undefined;
}

function handleEvent(qc: QueryClient, raw: string) {
  let ev: ServerEvent;
  try {
    ev = JSON.parse(raw) as ServerEvent;
  } catch {
    return;
  }
  const p = (ev.payload ?? {}) as Record<string, unknown>;
  // 红点/聚合几乎总受影响(dashboard_changed 亦经此刷新;桌面托盘角标后置)。
  qc.invalidateQueries({ queryKey: qk.dashboard });
  switch (ev.type) {
    case "certificate_status_changed": {
      qc.invalidateQueries({ queryKey: qk.certificates });
      qc.invalidateQueries({ queryKey: qk.domains }); // 域名"证书态投影"随之变
      const id = idOf(p, "certificateId");
      if (id) qc.invalidateQueries({ queryKey: qk.certificate(id) });
      break;
    }
    case "task_status_changed": {
      qc.invalidateQueries({ queryKey: qk.tasks });
      qc.invalidateQueries({ queryKey: qk.certificates });
      const tid = idOf(p, "taskId");
      if (tid) qc.invalidateQueries({ queryKey: qk.task(tid) });
      const cid = idOf(p, "certificateId");
      if (cid) qc.invalidateQueries({ queryKey: qk.certificate(cid) });
      break;
    }
    case "task_log_appended": {
      const tid = idOf(p, "taskId");
      if (tid) {
        qc.invalidateQueries({ queryKey: qk.taskLogs(tid) });
        qc.invalidateQueries({ queryKey: qk.task(tid) });
      }
      break;
    }
    case "root_ca_status_changed": {
      qc.invalidateQueries({ queryKey: qk.rootCas });
      const id = idOf(p, "rootCaId");
      if (id) qc.invalidateQueries({ queryKey: qk.rootCa(id) });
      break;
    }
    case "challenge_status_changed": {
      // payload 无 certificateId → 失效挑战根键(覆盖所有按证书维度的列表)+ 具体挑战详情。
      qc.invalidateQueries({ queryKey: qk.challenges });
      const chId = idOf(p, "challengeId");
      if (chId) qc.invalidateQueries({ queryKey: qk.challenge(chId) });
      const tid = idOf(p, "taskId");
      if (tid) qc.invalidateQueries({ queryKey: qk.task(tid) });
      break;
    }
    case "acme_account_status_changed":
      qc.invalidateQueries({ queryKey: qk.acmeAccounts });
      break;
    case "dashboard_changed":
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
      // 重连兜底:主动全量重取 dashboard + 关键列表(弥补断线期间可能丢失的事件,common/events §5)
      qc.invalidateQueries({ queryKey: qk.dashboard });
      qc.invalidateQueries({ queryKey: qk.certificates });
      qc.invalidateQueries({ queryKey: qk.tasks });
      qc.invalidateQueries({ queryKey: qk.domains });
      qc.invalidateQueries({ queryKey: qk.rootCas });
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
