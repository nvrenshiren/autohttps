import type { LucideIcon } from "lucide-react";
import { PageHeader } from "@/components/shared/page-header";
import { EmptyState } from "@/components/shared/states";
import { Card } from "@/components/ui/card";

/** 里程碑1 占位页(acme / local-ca / tasks 页面建设中;对应后端读取接口已就绪)。 */
export function ComingSoonPage({
  title,
  description,
  Icon,
}: {
  title: string;
  description?: string;
  Icon: LucideIcon;
}) {
  return (
    <div className="p-4 sm:p-6">
      <PageHeader title={title} />
      <Card>
        <EmptyState Icon={Icon} title={`${title} · 建设中`} description={description} />
      </Card>
    </div>
  );
}
