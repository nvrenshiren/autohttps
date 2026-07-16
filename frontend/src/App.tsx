import { Route, Routes } from "react-router";
import { AppShell } from "@/components/app-shell";
import { Toaster } from "@/components/ui/sonner";
import { useServerEvents } from "@/hooks/use-server-events";
import { useUiStore } from "@/stores/ui";
import { DashboardPage } from "@/pages/dashboard";
import { DomainsListPage } from "@/pages/domains-list";
import { DomainDetailPage } from "@/pages/domain-detail";
import { CertificatesListPage } from "@/pages/certificates-list";
import { CertificateDetailPage } from "@/pages/certificate-detail";
import { SettingsPage } from "@/pages/settings";
import { ComingSoonPage } from "@/pages/coming-soon";
import { NotFoundPage } from "@/pages/not-found";
import { BadgeCheck, Landmark, ListChecks } from "lucide-react";

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
          <Route path="/certificates/:id" element={<CertificateDetailPage />} />
          <Route
            path="/acme"
            element={
              <ComingSoonPage
                title="ACME 账户"
                Icon={BadgeCheck}
                description="配置 ACME 账户、DNS-01 / HTTP-01 验证向导与挑战管理。后端读取接口已就绪,页面建设中。"
              />
            }
          />
          <Route
            path="/local-ca"
            element={
              <ComingSoonPage
                title="根 CA"
                Icon={Landmark}
                description="自签根 CA 的创建 / 导入 / 导出与内网证书概览。后端读取接口已就绪,页面建设中。"
              />
            }
          />
          <Route
            path="/tasks"
            element={
              <ComingSoonPage
                title="任务与历史"
                Icon={ListChecks}
                description="签发 / 续签 / 吊销任务的队列、历史、执行日志与重试链。后端读取接口已就绪,页面建设中。"
              />
            }
          />
          <Route path="/settings" element={<SettingsPage />} />
          <Route path="*" element={<NotFoundPage />} />
        </Routes>
      </AppShell>
      <Toaster theme={theme} />
    </>
  );
}
