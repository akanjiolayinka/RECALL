import { Construction } from "lucide-react";

import { EmptyState } from "@/components/ui/empty-state";

/** Honest placeholder for screens whose functionality has not been implemented yet. */
export function NotBuiltYet({ description }: { description: string }) {
  return <EmptyState icon={Construction} title="Not built yet" description={description} />;
}
