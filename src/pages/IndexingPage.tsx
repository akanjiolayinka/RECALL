import { Activity, Folder } from "lucide-react";

import { PageHeader } from "@/components/layout/PageHeader";
import { describeScan, isScanning } from "@/components/indexing/ScanProgress";
import { EmptyState } from "@/components/ui/empty-state";
import { ErrorMessage } from "@/components/ui/error-message";
import { Progress } from "@/components/ui/progress";
import { useLocations } from "@/hooks/useLocations";
import { errorMessage, type Location } from "@/lib/api/client";
import { formatCount } from "@/lib/format";

function Stat({ label, value }: { label: string; value: number }) {
  return (
    <div>
      <p className="text-lg font-semibold tabular-nums">{formatCount(value)}</p>
      <p className="text-xs text-muted-foreground">{label}</p>
    </div>
  );
}

function LocationProgress({ location }: { location: Location }) {
  const scan = location.scan;
  const percent =
    scan?.state === "done"
      ? 100
      : scan?.state === "hashing" && scan.filesFound > 0
        ? (scan.filesProcessed / scan.filesFound) * 100
        : null;

  return (
    <li className="space-y-3 rounded-xl border bg-card p-4">
      <div className="flex items-center gap-3">
        <Folder className="size-4 text-muted-foreground" aria-hidden />
        <p className="font-medium">{location.name}</p>
        <p className="truncate text-xs text-muted-foreground" title={location.path}>
          {location.path}
        </p>
      </div>
      {scan?.state === "failed" ? (
        <ErrorMessage message={describeScan(scan)} />
      ) : (
        <>
          <Progress value={percent} label={`Progress for ${location.name}`} />
          <p className="text-sm">{describeScan(scan, location.fileCount)}</p>
          {isScanning(scan) && scan?.currentFile && (
            <p className="truncate text-xs text-muted-foreground">Now: {scan.currentFile}</p>
          )}
        </>
      )}
      {scan && (
        <div className="grid grid-cols-4 gap-4 border-t pt-3">
          <Stat label="Files found" value={scan.filesFound} />
          <Stat label="Files checked" value={scan.filesProcessed} />
          <Stat label="Couldn't read" value={scan.filesFailed} />
          <Stat label="Skipped (no access)" value={scan.unreadable} />
        </div>
      )}
    </li>
  );
}

export function IndexingPage() {
  const locations = useLocations();

  return (
    <div className="flex flex-col gap-6">
      <PageHeader title="Indexing" description="Progress as Recall reads files on this computer." />
      {locations.error ? (
        <ErrorMessage message={errorMessage(locations.error)} />
      ) : !locations.data ? (
        <p className="text-sm text-muted-foreground">Loading…</p>
      ) : locations.data.length === 0 ? (
        <EmptyState
          icon={Activity}
          title="Nothing to index yet"
          description="Add a folder in Library and its progress will appear here."
        />
      ) : (
        <>
          <ul className="flex flex-col gap-3">
            {locations.data.map((location) => (
              <LocationProgress key={location.id} location={location} />
            ))}
          </ul>
          <p className="text-xs text-muted-foreground">
            Recall finds supported files and fingerprints them so it can spot changes. Each time it
            starts, it checks your folders again; files that haven't changed aren't re-read. Reading
            the text inside files arrives in Milestone 5.
          </p>
        </>
      )}
    </div>
  );
}
