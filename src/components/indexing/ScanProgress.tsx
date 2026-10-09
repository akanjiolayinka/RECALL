import { Progress } from "@/components/ui/progress";
import type { ScanStatus } from "@/lib/api/client";
import { formatCount, plural } from "@/lib/format";
import { cn } from "@/lib/utils";

/** What the index holds for a folder (from the database, not one scan). */
export interface IndexCounts {
  fileCount: number;
  readCount: number;
  failedCount: number;
}

/** e.g. "9 files · 8 read · 1 couldn't be read". */
export function describeCounts({ fileCount, readCount, failedCount }: IndexCounts): string {
  const parts = [plural(fileCount, "file")];
  if (readCount > 0) parts.push(`${formatCount(readCount)} read`);
  if (failedCount > 0) parts.push(`${formatCount(failedCount)} couldn't be read`);
  return parts.join(" · ");
}

/**
 * One-line description of a folder: the current step while a scan runs,
 * otherwise what the index holds.
 */
export function describeScan(scan: ScanStatus | null, counts: IndexCounts): string {
  switch (scan?.state) {
    case "discovering":
      return `Looking for files… ${plural(scan.filesFound, "file")} found`;
    case "hashing":
      return `Checking ${formatCount(scan.filesProcessed)} of ${plural(scan.filesFound, "file")}`;
    case "reading":
      return `Reading ${formatCount(scan.filesRead)} of ${plural(scan.filesToRead, "document")}`;
    case "embedding":
      return `Understanding ${formatCount(scan.passagesEmbedded)} of ${plural(scan.passagesToEmbed, "passage")}`;
    case "failed":
      return scan.error ?? "Scanning failed.";
    default:
      return scan || counts.fileCount > 0 ? describeCounts(counts) : "Waiting to scan";
  }
}

export function isScanning(scan: ScanStatus | null): boolean {
  return (
    scan?.state === "discovering" ||
    scan?.state === "hashing" ||
    scan?.state === "reading" ||
    scan?.state === "embedding"
  );
}

/** 0–100 progress of the current step, or null when the amount is unknown. */
export function scanPercent(scan: ScanStatus | null): number | null {
  if (scan?.state === "done") return 100;
  if (scan?.state === "hashing" && scan.filesFound > 0) return (scan.filesProcessed / scan.filesFound) * 100;
  if (scan?.state === "reading" && scan.filesToRead > 0) return (scan.filesRead / scan.filesToRead) * 100;
  if (scan?.state === "embedding" && scan.passagesToEmbed > 0)
    return (scan.passagesEmbedded / scan.passagesToEmbed) * 100;
  return null;
}

/** Status line plus progress bar while a scan is running. */
interface ScanProgressProps {
  scan: ScanStatus | null;
  counts: IndexCounts;
  className?: string;
}

export function ScanProgress({ scan, counts, className }: ScanProgressProps) {
  const percent = scanPercent(scan);

  return (
    <div className={cn("space-y-1.5", className)}>
      <p className={cn("text-xs", scan?.state === "failed" ? "text-destructive" : "text-muted-foreground")}>
        {describeScan(scan, counts)}
      </p>
      {isScanning(scan) && <Progress value={percent} label="Scan progress" />}
    </div>
  );
}
