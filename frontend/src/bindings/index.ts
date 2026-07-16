/**
 * TS 类型绑定 —— Rust 是唯一真相,本文件是其投影(TECH §4.2)。
 *
 * 机制:core/api 的枚举与 DTO 均 `#[derive(ts_rs::TS)]`;本文件为其提交进版本库的投影
 * (契约允许"生成产物纳入版本库或构建期生成")。**前端只从这里 import 枚举/类型,禁在别处
 * 硬写状态字面量**(§4.2 / protocolLint L1)——唯一例外是设计系统 §3.2 的 StatusBadge 映射表
 * (`@/lib/status`),它是状态→呈现的单一入口。wire 值严格等于 TECH §4.3。
 */

// ============ 枚举(§4.3 wire 值)============

export type CertificateStatus =
  | "pending_issue"
  | "issuing"
  | "issue_failed"
  | "valid"
  | "expiring_soon"
  | "renewing"
  | "renewal_failed"
  | "expired"
  | "revoking"
  | "revoked";

export type IssuanceMethod = "acme" | "self_signed";

export type TaskStatus = "queued" | "running" | "succeeded" | "failed" | "cancelled";
export type TaskType = "issue" | "renew" | "revoke";
export type TaskTrigger = "manual" | "auto" | "cleanup";

export type AcmeAccountStatus =
  | "unconfigured"
  | "registering"
  | "registered"
  | "registration_failed";

export type ChallengeStatus =
  | "pending"
  | "awaiting_manual"
  | "validating"
  | "passed"
  | "failed"
  | "cancelled";

export type ValidationMethod = "http_01" | "dns_01";
export type RootCaStatus = "active" | "expired";
export type RunMode = "desktop" | "server";

export type EventType =
  | "certificate_status_changed"
  | "task_status_changed"
  | "task_log_appended"
  | "challenge_status_changed"
  | "acme_account_status_changed"
  | "root_ca_status_changed"
  | "dashboard_changed";

// 领域错误码(前端按 code 分支;message 仅展示)。仅列出前端可能分支的常用码。
export type ErrorCode =
  | "validation_failed"
  | "not_found"
  | "internal_error"
  | "not_implemented"
  | "cert_not_found"
  | "cert_in_progress_cannot_delete"
  | "cert_not_exportable"
  | "invalid_cert_state"
  | "no_domains_specified"
  | "invalid_domain_reference"
  | "multiple_wildcards_not_allowed"
  | "domain_validation_method_required"
  | "wildcard_requires_dns01"
  | "issuance_source_conflict"
  | "acme_account_required"
  | "invalid_acme_account_reference"
  | "acme_account_not_registered"
  | "root_ca_required"
  | "invalid_root_ca_reference"
  | "root_ca_expired"
  | "key_export_not_acknowledged"
  | "domain_not_found"
  | "domain_already_exists"
  | "domain_has_certificates"
  | "hostname_immutable"
  | "acme_account_not_found"
  | "challenge_not_found"
  | "http01_config_not_found"
  | "account_state_invalid"
  | "challenge_not_awaiting_manual"
  | "challenge_not_retryable"
  | "tos_not_agreed"
  | "invalid_directory_url"
  | "not_dns01_challenge"
  | "root_ca_not_found"
  | "invalid_validity_period"
  | "import_key_mismatch"
  | "import_invalid_certificate"
  | "import_key_decryption_failed"
  | "storage_path_read_only"
  | "setting_not_applicable"
  | "task_not_found"
  | "task_not_retryable"
  | "task_not_cancellable"
  | "certificate_deleted"
  | (string & {}); // 兜底:未来新增码不至于类型错

// ============ 通用 ============

export interface Page<T> {
  items: T[];
  page: number;
  pageSize: number;
  total: number;
}

export interface ApiErrorEnvelope {
  error: { code: ErrorCode; message: string; details?: unknown };
}

export interface AppInfo {
  runMode: RunMode;
  appVersion: string;
}

// ============ certificates ============

export interface DomainRef {
  id: string;
  hostname: string;
  isWildcard: boolean;
}

export interface CertificateSummary {
  id: string;
  status: CertificateStatus;
  issuanceMethod: IssuanceMethod;
  domains: DomainRef[];
  serialNumber: string | null;
  notBefore: string | null;
  notAfter: string | null;
  daysUntilExpiry: number | null;
  isExportable: boolean;
  lastError: string | null;
  updatedAt: string;
}

export interface AcmeAccountRef {
  id: string;
  caLabel: string | null;
  environment: string | null;
}

export interface RootCaRef {
  id: string;
  name: string;
}

export interface CertificateDetail extends CertificateSummary {
  fingerprint: string | null;
  issuedAt: string | null;
  createdAt: string;
  acmeAccount: AcmeAccountRef | null;
  rootCa: RootCaRef | null;
  activeTaskId: string | null;
}

export interface IssueCertificateRequest {
  issuanceMethod: IssuanceMethod;
  domainIds: string[];
  acmeAccountId?: string;
  rootCaId?: string;
}

// ============ domains ============

export interface DomainSummary {
  id: string;
  hostname: string;
  isWildcard: boolean;
  groupName: string | null;
  remark: string | null;
  validationMethod: ValidationMethod | null;
  certificateCount: number;
  worstCertificateStatus: CertificateStatus | null;
  updatedAt: string;
}

export interface DomainCertificateRef {
  id: string;
  status: CertificateStatus;
  issuanceMethod: IssuanceMethod;
  notAfter: string | null;
  daysUntilExpiry: number | null;
}

export interface DomainDetail extends DomainSummary {
  createdAt: string;
  certificates: DomainCertificateRef[];
}

export interface CreateDomainRequest {
  hostname: string;
  groupName?: string;
  remark?: string;
  validationMethod?: ValidationMethod;
}

export interface UpdateDomainRequest {
  groupName?: string | null;
  remark?: string | null;
  validationMethod?: ValidationMethod | null;
}

// ============ settings ============

export interface SettingsView {
  renewalAdvanceDays: number;
  autoRenewEnabled: boolean;
  defaultAcmeAccountId: string | null;
  autostartEnabled: boolean | null;
  listenAddress: string | null;
  listenPort: number | null;
  dataStoragePath: string;
  updatedAt: string;
}

export interface UpdateSettingsRequest {
  renewalAdvanceDays?: number;
  autoRenewEnabled?: boolean;
  defaultAcmeAccountId?: string | null;
  autostartEnabled?: boolean;
  listenAddress?: string;
  listenPort?: number;
}

// ============ dashboard ============

export interface DashboardMetrics {
  totalCount: number;
  expiringSoonCount: number;
  failedCount: number;
}

export interface PendingCertItem {
  certificateId: string;
  status: CertificateStatus;
  domains: string[];
  issuanceMethod: IssuanceMethod;
  notAfter: string | null;
  daysUntilExpiry: number | null;
  latestTaskId: string | null;
}

export interface DashboardOverview {
  metrics: DashboardMetrics;
  pendingCount: number;
  pendingItems: PendingCertItem[];
}

// ============ tasks ============

export interface TaskSummary {
  id: string;
  certificateId: string;
  certificateDeleted: boolean;
  certificateDomains: string[] | null;
  taskType: TaskType;
  trigger: TaskTrigger;
  status: TaskStatus;
  attemptNumber: number;
  queuedAt: string;
  startedAt: string | null;
  finishedAt: string | null;
  resultSummary: string | null;
  failureReason: string | null;
}

export interface TaskCertificateRef {
  id: string;
  status: CertificateStatus;
  domains: string[];
}

export interface TaskDetail extends TaskSummary {
  parentTaskId: string | null;
  childTaskIds: string[];
  certificate: TaskCertificateRef | null;
  createdAt: string;
  updatedAt: string;
}

export interface TaskLogEntry {
  id: string;
  taskId: string;
  seq: number;
  loggedAt: string;
  level: string;
  message: string;
}

// ============ acme ============

export interface AcmeAccountSummary {
  id: string;
  directoryUrl: string;
  caLabel: string | null;
  environment: string | null;
  contactEmail: string;
  status: AcmeAccountStatus;
  isDefault: boolean;
  certificateCount: number;
  registeredAt: string | null;
  lastError: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface AcmeAccountDetail extends AcmeAccountSummary {
  caAccountUrl: string | null;
  tosAgreed: boolean;
}

export interface Http01Config {
  domainId: string;
  webrootPath: string;
  updatedAt: string;
}

export interface ChallengeSummary {
  id: string;
  taskId: string;
  certificateId: string;
  domainId: string;
  domainHostname: string | null;
  validationMethod: ValidationMethod;
  status: ChallengeStatus;
  failedReason: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface ChallengeDetail extends ChallengeSummary {
  dnsTxtName: string | null;
  dnsTxtValue: string | null;
  httpFilePath: string | null;
  httpFileContent: string | null;
}

// ============ local-ca ============

export interface RootCaSummary {
  id: string;
  name: string;
  status: RootCaStatus;
  creationMethod: string;
  notBefore: string;
  notAfter: string;
  daysUntilExpiry: number;
  serialNumber: string | null;
  fingerprint: string | null;
  issuedCertificateCount: number;
  createdAt: string;
  updatedAt: string;
}

export interface RootCaDetail extends RootCaSummary {
  certPem: string;
}

export interface CreateRootCaRequest {
  name: string;
  validityDays: number;
}

export interface ImportRootCaRequest {
  name: string;
  certPem: string;
  privateKeyPem: string;
  keyPassphrase?: string;
}

// ============ SSE 事件 ============

export interface ServerEvent<T = unknown> {
  type: EventType;
  at: string;
  payload: T;
}

// ============ 运行时枚举清单(枚举投影的运行时伴生;供筛选下拉等,禁在别处硬写)============

export const CERTIFICATE_STATUSES: CertificateStatus[] = [
  "pending_issue",
  "issuing",
  "issue_failed",
  "valid",
  "expiring_soon",
  "renewing",
  "renewal_failed",
  "expired",
  "revoking",
  "revoked",
];

export const ISSUANCE_METHODS: IssuanceMethod[] = ["acme", "self_signed"];

export const TASK_STATUSES: TaskStatus[] = [
  "queued",
  "running",
  "succeeded",
  "failed",
  "cancelled",
];

export const TASK_TYPES: TaskType[] = ["issue", "renew", "revoke"];

export const TASK_TRIGGERS: TaskTrigger[] = ["manual", "auto", "cleanup"];

export const ROOT_CA_STATUSES: RootCaStatus[] = ["active", "expired"];
