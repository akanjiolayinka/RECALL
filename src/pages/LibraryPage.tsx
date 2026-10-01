import { NotBuiltYet } from "@/components/layout/NotBuiltYet";
import { PageHeader } from "@/components/layout/PageHeader";

export function LibraryPage() {
  return (
    <div className="flex flex-col gap-6">
      <PageHeader title="Library" description="The folders and files Recall has indexed." />
      <NotBuiltYet description="Choosing a folder and browsing indexed files arrive in Milestones 2–4." />
    </div>
  );
}
