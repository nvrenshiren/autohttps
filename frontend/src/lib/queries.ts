/**
 * react-query hooks(服务端态单一来源,TECH §1.4)。SSE 收到事件后 invalidate 对应 key(见
 * use-server-events)。列表用 keepPreviousData 平滑翻页。
 */
import {
  keepPreviousData,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { api, qs } from "@/lib/api";
import { useUiStore } from "@/stores/ui";
import type {
  AcmeAccountDetail,
  AcmeAccountSummary,
  AppInfo,
  CertificateDetail,
  CertificateSummary,
  ChallengeDetail,
  ChallengeSummary,
  CreateDomainRequest,
  CreateRootCaRequest,
  DashboardOverview,
  DomainDetail,
  DomainSummary,
  ImportRootCaRequest,
  IssueCertificateRequest,
  Page,
  RegisterAcmeAccountRequest,
  RootCaDetail,
  RootCaSummary,
  SettingsView,
  TaskDetail,
  TaskLogEntry,
  TaskSummary,
  UpdateDomainRequest,
  UpdateSettingsRequest,
} from "@/bindings";

/**
 * SSE 断线时的低频轮询兜底(common/events.md §5)。SSE 连上时纯靠事件 `invalidate`(返回 false 不轮询);
 * 断开且未及时重连时对"关键列表"(dashboard / 证书 / 任务)启用 refetchInterval,恢复后自动停轮询。
 */
const FALLBACK_POLL_MS = 15_000;
function useFallbackInterval(): number | false {
  return useUiStore((s) => (s.sseConnected ? false : FALLBACK_POLL_MS));
}

export const qk = {
  appInfo: ["app-info"] as const,
  dashboard: ["dashboard"] as const,
  certificates: ["certificates"] as const,
  certificate: (id: string) => ["certificate", id] as const,
  domains: ["domains"] as const,
  domain: (id: string) => ["domain", id] as const,
  settings: ["settings"] as const,
  tasks: ["tasks"] as const,
  task: (id: string) => ["task", id] as const,
  taskLogs: (id: string) => ["task-logs", id] as const,
  rootCas: ["root-cas"] as const,
  rootCa: (id: string) => ["root-ca", id] as const,
  acmeAccounts: ["acme-accounts"] as const,
  // 挑战:根键 ["challenges"] 供 SSE 前缀失效(payload 无 certificateId,按证书维度的列表全部重取)。
  challenges: ["challenges"] as const,
  challengesByCert: (certificateId: string) => ["challenges", certificateId] as const,
  challenge: (id: string) => ["challenge", id] as const,
  syncConfig: ["sync-config"] as const,
  syncBackups: ["sync-backups"] as const,
};

// ---------- app-info / dashboard ----------

export function useAppInfo() {
  return useQuery({
    queryKey: qk.appInfo,
    queryFn: () => api.get<AppInfo>("/app-info"),
    staleTime: Infinity,
  });
}

export function useDashboard() {
  return useQuery({
    queryKey: qk.dashboard,
    queryFn: () => api.get<DashboardOverview>("/dashboard"),
    refetchInterval: useFallbackInterval(),
  });
}

// ---------- certificates ----------

export interface CertFilter {
  page?: number;
  pageSize?: number;
  status?: string;
  issuanceMethod?: string;
  domain?: string;
  sort?: string;
  order?: string;
}

export function useCertificates(f: CertFilter) {
  return useQuery({
    queryKey: [...qk.certificates, f],
    queryFn: () =>
      api.get<Page<CertificateSummary>>(
        "/certificates" +
          qs({
            page: f.page,
            pageSize: f.pageSize,
            status: f.status,
            issuanceMethod: f.issuanceMethod,
            domain: f.domain,
            sort: f.sort,
            order: f.order,
          }),
      ),
    placeholderData: keepPreviousData,
    refetchInterval: useFallbackInterval(),
  });
}

export function useCertificate(id: string) {
  return useQuery({
    queryKey: qk.certificate(id),
    queryFn: () => api.get<CertificateDetail>(`/certificates/${id}`),
    refetchInterval: useFallbackInterval(),
  });
}

export function useCreateCertificate() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: IssueCertificateRequest) =>
      api.post<CertificateDetail>("/certificates", body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: qk.certificates });
      qc.invalidateQueries({ queryKey: qk.dashboard });
    },
  });
}

export function useDeleteCertificate() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.del(`/certificates/${id}`),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: qk.certificates });
      qc.invalidateQueries({ queryKey: qk.dashboard });
      qc.invalidateQueries({ queryKey: qk.domains });
      qc.invalidateQueries({ queryKey: qk.tasks });
    },
  });
}

// ---------- domains ----------

export interface DomainFilter {
  page?: number;
  pageSize?: number;
  group?: string;
  hostname?: string;
  sort?: string;
  order?: string;
}

export function useDomains(f: DomainFilter) {
  return useQuery({
    queryKey: [...qk.domains, f],
    queryFn: () =>
      api.get<Page<DomainSummary>>(
        "/domains" +
          qs({
            page: f.page,
            pageSize: f.pageSize,
            group: f.group,
            hostname: f.hostname,
            sort: f.sort,
            order: f.order,
          }),
      ),
    placeholderData: keepPreviousData,
  });
}

export function useDomain(id: string) {
  return useQuery({
    queryKey: qk.domain(id),
    queryFn: () => api.get<DomainDetail>(`/domains/${id}`),
  });
}

export function useCreateDomain() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: CreateDomainRequest) => api.post<DomainDetail>("/domains", body),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.domains }),
  });
}

export function useUpdateDomain(id: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: UpdateDomainRequest) => api.patch<DomainDetail>(`/domains/${id}`, body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: qk.domains });
      qc.invalidateQueries({ queryKey: qk.domain(id) });
    },
  });
}

export function useDeleteDomain() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.del(`/domains/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.domains }),
  });
}

// ---------- settings ----------

export function useSettings() {
  return useQuery({
    queryKey: qk.settings,
    queryFn: () => api.get<SettingsView>("/settings"),
  });
}

export function useUpdateSettings() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: UpdateSettingsRequest) => api.patch<SettingsView>("/settings", body),
    onSuccess: (data) => qc.setQueryData(qk.settings, data),
  });
}

// ---------- sync(WebDAV 备份)----------

export interface SyncConfigView {
  configured: boolean;
  /** 服务器地址(不含远程目录) */
  serverUrl: string | null;
  /** 远程目录(备份与其他项目隔离) */
  remoteDir: string | null;
  /** 拼好的完整远端目录 URL(实际请求目标) */
  baseUrl: string | null;
  username: string | null;
  passwordSet: boolean;
  lastBackupAt: string | null;
  lastBackupResult: string | null;
  lastBackupError: string | null;
}

export interface RemoteBackupItem {
  name: string;
  size: number | null;
  modified: string | null;
}

export interface RestoreOutcome {
  restoredFrom: string;
  backupCreatedAt: string;
  secretsRestored: number;
  requiresRestart: boolean;
}

export function useSyncConfig() {
  return useQuery({
    queryKey: qk.syncConfig,
    queryFn: () => api.get<SyncConfigView>("/sync/webdav-config"),
  });
}

export function useSaveSyncConfig() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: {
      serverUrl: string;
      remoteDir?: string;
      username: string;
      password?: string;
    }) => api.put<SyncConfigView>("/sync/webdav-config", body),
    onSuccess: (data) => qc.setQueryData(qk.syncConfig, data),
  });
}

export function useDeleteSyncConfig() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => api.del("/sync/webdav-config"),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.syncConfig }),
  });
}

export function useTestSyncConnection() {
  return useMutation({
    mutationFn: () => api.post<{ ok: boolean }>("/sync/test"),
  });
}

export function useBackupNow() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (passphrase: string) =>
      api.post<RemoteBackupItem>("/sync/backup", { passphrase }),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.syncConfig }),
  });
}

export function useRemoteBackups(enabled: boolean) {
  return useQuery({
    queryKey: qk.syncBackups,
    queryFn: () => api.get<RemoteBackupItem[]>("/sync/backups"),
    enabled,
  });
}

export function useRestoreBackup() {
  return useMutation({
    mutationFn: (body: { remoteName: string; passphrase: string }) =>
      api.post<RestoreOutcome>("/sync/restore", body),
  });
}

// ---------- tasks ----------

export interface TaskFilter {
  page?: number;
  pageSize?: number;
  taskType?: string;
  status?: string; // 逗号分隔多值;`queued,running` 即队列
  certificateId?: string;
  trigger?: string;
  dateFrom?: string;
  dateTo?: string;
  sort?: string;
  order?: string;
}

export function useTasks(f: TaskFilter) {
  return useQuery({
    queryKey: [...qk.tasks, f],
    queryFn: () =>
      api.get<Page<TaskSummary>>(
        "/tasks" +
          qs({
            page: f.page,
            pageSize: f.pageSize,
            taskType: f.taskType,
            status: f.status,
            certificateId: f.certificateId,
            trigger: f.trigger,
            dateFrom: f.dateFrom,
            dateTo: f.dateTo,
            sort: f.sort,
            order: f.order,
          }),
      ),
    placeholderData: keepPreviousData,
    refetchInterval: useFallbackInterval(),
  });
}

export function useTask(id: string) {
  return useQuery({
    queryKey: qk.task(id),
    queryFn: () => api.get<TaskDetail>(`/tasks/${id}`),
    refetchInterval: useFallbackInterval(),
  });
}

export function useTaskLogs(id: string) {
  return useQuery({
    queryKey: qk.taskLogs(id),
    queryFn: () => api.get<Page<TaskLogEntry>>(`/tasks/${id}/logs` + qs({ pageSize: 500 })),
    refetchInterval: useFallbackInterval(),
  });
}

// ---------- local-ca ----------

export interface RootCaFilter {
  page?: number;
  pageSize?: number;
  status?: string;
  sort?: string;
  order?: string;
}

export function useRootCas(f: RootCaFilter) {
  return useQuery({
    queryKey: [...qk.rootCas, f],
    queryFn: () =>
      api.get<Page<RootCaSummary>>(
        "/root-cas" +
          qs({ page: f.page, pageSize: f.pageSize, status: f.status, sort: f.sort, order: f.order }),
      ),
    placeholderData: keepPreviousData,
  });
}

export function useRootCa(id: string) {
  return useQuery({
    queryKey: qk.rootCa(id),
    queryFn: () => api.get<RootCaDetail>(`/root-cas/${id}`),
  });
}

export function useCreateRootCa() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: CreateRootCaRequest) => api.post<RootCaDetail>("/root-cas", body),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.rootCas }),
  });
}

export function useImportRootCa() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: ImportRootCaRequest) => api.post<RootCaDetail>("/root-cas/import", body),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.rootCas }),
  });
}

// ---------- acme accounts(签发页方式=公共 ACME 时选账户;账户/向导管理归 acme 模块)----------

export function useAcmeAccounts(status?: string) {
  return useQuery({
    queryKey: [...qk.acmeAccounts, { status }],
    queryFn: () =>
      api.get<Page<AcmeAccountSummary>>("/acme/accounts" + qs({ status, pageSize: 100 })),
    refetchInterval: useFallbackInterval(),
  });
}

export function useRegisterAcmeAccount() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: RegisterAcmeAccountRequest) =>
      api.post<AcmeAccountDetail>("/acme/accounts", body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: qk.acmeAccounts });
      qc.invalidateQueries({ queryKey: qk.dashboard });
    },
  });
}

export function useUpdateAcmeAccountEmail() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { id: string; contactEmail: string }) =>
      api.patch<AcmeAccountDetail>(`/acme/accounts/${vars.id}`, { contactEmail: vars.contactEmail }),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.acmeAccounts }),
  });
}

export function useRetryAcmeAccount() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.post<AcmeAccountDetail>(`/acme/accounts/${id}/retry`),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: qk.acmeAccounts });
      qc.invalidateQueries({ queryKey: qk.dashboard });
    },
  });
}

export function useDeleteAcmeAccount() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.del(`/acme/accounts/${id}`),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: qk.acmeAccounts });
      qc.invalidateQueries({ queryKey: qk.certificates }); // 引用置空(SET NULL)
      qc.invalidateQueries({ queryKey: qk.settings }); // 默认账户指向可能被清空
      qc.invalidateQueries({ queryKey: qk.dashboard });
    },
  });
}

// ---------- acme challenges(验证方式向导:按证书维度看各域名挑战 + DNS-01 确认 / 失败重试)----------

export function useChallenges(certificateId: string) {
  return useQuery({
    queryKey: qk.challengesByCert(certificateId),
    queryFn: () =>
      api.get<Page<ChallengeSummary>>(
        "/acme/challenges" +
          qs({ certificateId, pageSize: 100, sort: "createdAt", order: "asc" }),
      ),
    enabled: !!certificateId,
    refetchInterval: useFallbackInterval(),
  });
}

/** 挑战详情(DNS-01 取 TXT 名/值供复制)。仅按需(DNS-01)拉取,HTTP-01 摘要即够。 */
export function useChallenge(id: string, enabled = true) {
  return useQuery({
    queryKey: qk.challenge(id),
    queryFn: () => api.get<ChallengeDetail>(`/acme/challenges/${id}`),
    enabled: enabled && !!id,
    refetchInterval: useFallbackInterval(),
  });
}

export function useConfirmChallenge() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.post<ChallengeDetail>(`/acme/challenges/${id}/confirm`),
    onSuccess: (_data, id) => {
      qc.invalidateQueries({ queryKey: qk.challenges });
      qc.invalidateQueries({ queryKey: qk.challenge(id) });
      qc.invalidateQueries({ queryKey: qk.tasks });
      qc.invalidateQueries({ queryKey: qk.dashboard });
    },
  });
}

/** 失败重试(CT7)—— 后端重建订单派生新任务/新挑战,故一并失效证书/任务列表。 */
export function useRetryChallenge() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.post(`/acme/challenges/${id}/retry`),
    onSuccess: (_data, id) => {
      qc.invalidateQueries({ queryKey: qk.challenges });
      qc.invalidateQueries({ queryKey: qk.challenge(id) });
      qc.invalidateQueries({ queryKey: qk.certificates });
      qc.invalidateQueries({ queryKey: qk.tasks });
      qc.invalidateQueries({ queryKey: qk.dashboard });
    },
  });
}
