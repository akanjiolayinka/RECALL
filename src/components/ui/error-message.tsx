import { CircleAlert } from "lucide-react";

import { cn } from "@/lib/utils";

/** Inline error message. Pass user-facing text only (e.g. ApiError.message). */
export function ErrorMessage({ message, className }: { message: string; className?: string }) {
  return (
    <div
      role="alert"
      className={cn(
        "flex items-start gap-3 rounded-lg border border-destructive/30 bg-destructive/5 p-3 text-sm",
        className,
      )}
    >
      <CircleAlert className="mt-0.5 size-4 shrink-0 text-destructive" aria-hidden />
      <p>{message}</p>
    </div>
  );
}
