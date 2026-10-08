import type { ReactNode } from "react";

import { PageHeader } from "@/components/layout/PageHeader";
import { PrivacyFact } from "@/components/privacy/PrivacyFact";
import { ErrorMessage } from "@/components/ui/error-message";
import { useAiStatus } from "@/hooks/useAiStatus";
import { usePrivacyReport } from "@/hooks/usePrivacyReport";
import { errorMessage, type AiFeatureStatus } from "@/lib/api/client";
import { formatBytes, formatCount } from "@/lib/format";

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="space-y-2" aria-label={title}>
      <h2 className="text-sm font-semibold">{title}</h2>
      <dl className="divide-y rounded-xl border bg-card">{children}</dl>
    </section>
  );
}

function aiValue(feature: AiFeatureStatus | undefined) {
  if (!feature) return "…";
  return feature.available ? "Runs on this computer" : "Not installed";
}

function aiEvidence(feature: AiFeatureStatus | undefined, purpose: string) {
  if (!feature) return null;
  return feature.available
    ? `${purpose} uses ${feature.model}, loaded from files on this computer.`
    : feature.unavailableReason;
}

/**
 * Privacy facts. Every value is either measured from the running app or a
 * property of Recall's code checked by `npm run audit:privacy`; each says how.
 */
export function PrivacyPage() {
  const ai = useAiStatus();
  const report = usePrivacyReport();
  const error = ai.error ?? report.error;

  return (
    <div className="flex max-w-3xl flex-col gap-6">
      <PageHeader title="Privacy" description="What Recall does with your data, and how we know." />
      {error && <ErrorMessage message={errorMessage(error)} />}

      <Section title="AI">
        <PrivacyFact
          label="Reading text in images"
          value={aiValue(ai.data?.ocr)}
          positive={ai.data?.ocr.available}
          evidence={aiEvidence(ai.data?.ocr, "OCR")}
        />
        <PrivacyFact
          label="Meaning-based search"
          value={aiValue(ai.data?.semanticSearch)}
          positive={ai.data?.semanticSearch.available}
          evidence={aiEvidence(ai.data?.semanticSearch, "Search")}
        />
        <PrivacyFact
          label="Cloud AI services"
          value="None"
          positive
          evidence="Recall's code contains no calls to any online AI service. Checked by the privacy audit (npm run audit:privacy)."
        />
      </Section>

      <Section title="Internet">
        <PrivacyFact
          label="Uploads"
          value="Not possible"
          positive
          evidence="Recall's app contains no internet client on Windows, macOS or Linux, and its window is only allowed to talk to Recall itself (content security policy). Both are checked by the privacy audit."
        />
        <PrivacyFact
          label="Model downloads"
          value="Separate, one-time step"
          evidence="AI model files are downloaded once with npm run download-models, a setup script outside the app that verifies each file's checksum."
        />
      </Section>

      <Section title="What Recall stores">
        <PrivacyFact
          label="Index location"
          value={<span className="font-mono text-xs break-all">{report.data?.databasePath ?? "…"}</span>}
          evidence="A single database file on this computer. Deleting it resets Recall."
        />
        <PrivacyFact
          label="Index size"
          value={report.data ? formatBytes(report.data.databaseBytes) : "…"}
          evidence={
            report.data
              ? `${formatCount(report.data.folders)} folders, ${formatCount(report.data.files)} files, ` +
                `${formatCount(report.data.documents)} documents read, ${formatCount(report.data.passages)} searchable passages, ` +
                `${formatCount(report.data.embeddings)} embeddings.`
              : null
          }
        />
        <PrivacyFact
          label="Your original files"
          value="Read only"
          positive
          evidence="Recall reads your files to index them. It never changes, moves, copies or deletes them."
        />
      </Section>
    </div>
  );
}
