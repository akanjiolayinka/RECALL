import { NotBuiltYet } from "@/components/layout/NotBuiltYet";
import { PageHeader } from "@/components/layout/PageHeader";

export function IndexingPage() {
  return (
    <div className="flex flex-col gap-6">
      <PageHeader title="Indexing" description="Progress as Recall reads files on this computer." />
      <NotBuiltYet description="Indexing progress arrives once file scanning is built (Milestone 3)." />
    </div>
  );
}
