/**
 * 证书状态可用操作判定 —— **前端 UI 门控**(设计 H4)。是 core `domain/rules.rs` 的 TS 镜像:
 * 后端是权威强制方,前端据此禁用/启用按钮 + Tooltip 原因。转移规则权威:flows/certificates §2.3。
 */
import type { CertificateStatus } from "@/bindings";

export function isInProgress(s: CertificateStatus): boolean {
  return s === "issuing" || s === "renewing" || s === "revoking";
}

/** 可导出:非文件态(pending_issue / issuing / issue_failed)不可导出(§2.8)。 */
export function isExportable(s: CertificateStatus): boolean {
  return !(s === "pending_issue" || s === "issuing" || s === "issue_failed");
}

/** 可删除:非进行中态(§2.7)。 */
export function canDelete(s: CertificateStatus): boolean {
  return !isInProgress(s);
}

/** 可续签/再获取:valid / expiring_soon / expired / revoked(§2.4)。 */
export function canRenew(s: CertificateStatus): boolean {
  return s === "valid" || s === "expiring_soon" || s === "expired" || s === "revoked";
}

/** 可失败重试:issue_failed / renewal_failed(§2.5)。 */
export function canRetry(s: CertificateStatus): boolean {
  return s === "issue_failed" || s === "renewal_failed";
}

/** 可吊销:valid / expiring_soon / renewal_failed(§2.6,权威转移表不含 expired)。 */
export function canRevoke(s: CertificateStatus): boolean {
  return s === "valid" || s === "expiring_soon" || s === "renewal_failed";
}
