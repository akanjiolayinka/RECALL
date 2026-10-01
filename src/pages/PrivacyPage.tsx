import { NotBuiltYet } from "@/components/layout/NotBuiltYet";
import { PageHeader } from "@/components/layout/PageHeader";

export function PrivacyPage() {
  return (
    <div className="flex flex-col gap-6">
      <PageHeader title="Privacy" description="What Recall does with your data, measured from the running app." />
      <NotBuiltYet description="The privacy dashboard arrives in Milestone 12. It will only show values the app can actually verify — never placeholder numbers." />
    </div>
  );
}
