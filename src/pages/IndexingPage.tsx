import { Activity, Folder } from "lucide-react";

import { PageHeader } from "@/components/layout/PageHeader";
import { describeScan, isScanning, scanPercent } from "@/components/indexing/ScanProgress";
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
  const percent = scanPercent(scan);

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
        <div className="grid grid-cols-5 gap-4 border-t pt-3">
          <Stat label="Files found" value={scan.filesFound} />
          <Stat label="Files checked" value={scan.filesProcessed} />
          <Stat label="Documents read" value={scan.filesRead - scan.readFailed} />
          <Stat label="Couldn't read" value={scan.filesFailed + scan.readFailed} />
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
            Recall finds supported files, fingerprints them to spot changes, and reads their text
            (images with the local OCR model, when installed). Each time it starts it checks your
            folders again; files that haven't changed aren't re-read.
          </p>
        </>
      )}
    </div>
  );
}
