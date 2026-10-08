import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

interface PrivacyFactProps {
  label: string;
  value: ReactNode;
  /** How Recall knows this is true. */
  evidence: ReactNode;
  /** Highlights the value as a good privacy outcome. */
  positive?: boolean;
}

/** One verifiable statement on the Privacy page. */
export function PrivacyFact({ label, value, evidence, positive }: PrivacyFactProps) {
  return (
    <div className="grid grid-cols-[minmax(0,14rem)_1fr] gap-x-6 gap-y-1 px-5 py-4">
      <dt className="text-sm text-muted-foreground">{label}</dt>
      <dd className={cn("text-sm font-semibold", positive && "text-emerald-700 dark:text-emerald-400")}>
        {value}
      </dd>
      <dd className="col-start-2 text-xs text-muted-foreground">{evidence}</dd>
    </div>
  );
}
