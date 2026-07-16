import { FileQuestion } from "lucide-react";
import { Link } from "react-router";
import { EmptyState } from "@/components/shared/states";
import { Button } from "@/components/ui/button";

export function NotFoundPage() {
  return (
    <div className="p-6">
      <EmptyState
        Icon={FileQuestion}
        title="页面不存在"
        description="你访问的页面不存在或已移动。"
        action={
          <Button asChild>
            <Link to="/">返回总览</Link>
          </Button>
        }
      />
    </div>
  );
}
