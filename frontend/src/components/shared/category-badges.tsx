/** 分类标签(§3.5)—— 中性 outline,不占语义色(防彩虹汤)。 */
import { KeyRound, Landmark } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import type { IssuanceMethod, ValidationMethod } from "@/bindings";

export function IssuanceMethodBadge({ method }: { method: IssuanceMethod }) {
  return method === "acme" ? (
    <Badge variant="outline">
      <KeyRound className="size-3" />
      ACME
    </Badge>
  ) : (
    <Badge variant="outline">
      <Landmark className="size-3" />
      自签
    </Badge>
  );
}

export function ValidationMethodBadge({ method }: { method: ValidationMethod | null }) {
  if (!method) return <span className="text-muted-foreground">—</span>;
  return <Badge variant="outline">{method === "http_01" ? "HTTP-01" : "DNS-01"}</Badge>;
}

export function WildcardBadge() {
  return <Badge variant="outline">通配符</Badge>;
}
