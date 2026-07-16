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
import type {
  AcmeAccountSummary,
  AppInfo,
  CertificateDetail,
  CertificateSummary,
  CreateDomainRequest,
  CreateRootCaRequest,
  DashboardOverview,
  DomainDetail,
  DomainSummary,
  ImportRootCaRequest,
  IssueCertificateRequest,
  Page,
  RootCaDetail,
  RootCaSummary,
  SettingsView,
  TaskDetail,
  TaskLogEntry,
  TaskSummary,
  UpdateDomainRequest,
  UpdateSettingsRequest,
} from "@/bindings";

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
  });
}

export function useCertificate(id: string) {
  return useQuery({
    queryKey: qk.certificate(id),
    queryFn: () => api.get<CertificateDetail>(`/certificates/${id}`),
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
  });
}

export function useTask(id: string) {
  return useQuery({
    queryKey: qk.task(id),
    queryFn: () => api.get<TaskDetail>(`/tasks/${id}`),
  });
}

export function useTaskLogs(id: string) {
  return useQuery({
    queryKey: qk.taskLogs(id),
    queryFn: () => api.get<Page<TaskLogEntry>>(`/tasks/${id}/logs` + qs({ pageSize: 500 })),
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
  });
}
