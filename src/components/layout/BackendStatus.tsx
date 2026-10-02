import { useAppInfo } from "@/hooks/useAppInfo";
import { errorMessage } from "@/lib/api/client";
import { cn } from "@/lib/utils";

/** Small indicator showing whether the UI is connected to the Rust backend. */
export function BackendStatus() {
  const { data: info, error, isPending } = useAppInfo();

  const label = isPending
    ? "Connecting…"
    : error
      ? "Backend unavailable"
      : info.backend === "mock"
        ? "Mock data (dev only)"
        : `Local backend · v${info.version}`;

  const dotColor = isPending
    ? "bg-muted-foreground"
    : error
      ? "bg-destructive"
      : info.backend === "mock"
        ? "bg-amber-500"
        : "bg-emerald-500";

  return (
    <div
      className="flex items-center gap-2 text-xs text-muted-foreground"
      title={error ? errorMessage(error) : undefined}
    >
      <span className={cn("size-2 rounded-full", dotColor)} aria-hidden />
      <span>{label}</span>
    </div>
  );
}
