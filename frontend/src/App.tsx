import { Route, Routes } from "react-router";
import { AppShell } from "@/components/app-shell";
import { Toaster } from "@/components/ui/sonner";
import { useServerEvents } from "@/hooks/use-server-events";
import { useUiStore } from "@/stores/ui";
import { DashboardPage } from "@/pages/dashboard";
import { DomainsListPage } from "@/pages/domains-list";
import { DomainDetailPage } from "@/pages/domain-detail";
import { CertificatesListPage } from "@/pages/certificates-list";
import { CertificateIssuePage } from "@/pages/certificate-issue";
import { CertificateDetailPage } from "@/pages/certificate-detail";
import { ChallengeWizardPage } from "@/pages/challenge-wizard";
import { AcmeAccountsPage } from "@/pages/acme-accounts";
import { RootCaListPage } from "@/pages/local-ca-list";
import { RootCaCreatePage } from "@/pages/local-ca-create";
import { RootCaDetailPage } from "@/pages/local-ca-detail";
import { TasksListPage } from "@/pages/tasks-list";
import { TaskDetailPage } from "@/pages/task-detail";
import { SettingsPage } from "@/pages/settings";
import { NotFoundPage } from "@/pages/not-found";

export default function App() {
  useServerEvents();
  const theme = useUiStore((s) => s.theme);

  return (
    <>
      <AppShell>
        <Routes>
          <Route path="/" element={<DashboardPage />} />
          <Route path="/domains" element={<DomainsListPage />} />
          <Route path="/domains/:id" element={<DomainDetailPage />} />
          <Route path="/certificates" element={<CertificatesListPage />} />
          <Route path="/certificates/issue" element={<CertificateIssuePage />} />
          <Route path="/certificates/:id" element={<CertificateDetailPage />} />
          <Route path="/certificates/:id/challenges" element={<ChallengeWizardPage />} />
          <Route path="/acme" element={<AcmeAccountsPage />} />
          <Route path="/local-ca" element={<RootCaListPage />} />
          <Route path="/local-ca/new" element={<RootCaCreatePage />} />
          <Route path="/local-ca/:id" element={<RootCaDetailPage />} />
          <Route path="/tasks" element={<TasksListPage />} />
          <Route path="/tasks/:id" element={<TaskDetailPage />} />
          <Route path="/settings" element={<SettingsPage />} />
          <Route path="*" element={<NotFoundPage />} />
        </Routes>
      </AppShell>
      <Toaster theme={theme} />
    </>
  );
}
