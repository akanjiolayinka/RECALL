import { Progress } from "@/components/ui/progress";
import type { ScanStatus } from "@/lib/api/client";
import { formatCount, plural } from "@/lib/format";
import { cn } from "@/lib/utils";

/**
 * One-line, human-readable description of a folder scan.
 * `fileCount` (files already in the index) is shown when there is no scan
 * progress to report.
 */
export function describeScan(scan: ScanStatus | null, fileCount = 0): string {
  if (!scan) return fileCount > 0 ? `${plural(fileCount, "file")} in your index` : "Waiting to scan";
  switch (scan.state) {
    case "discovering":
      return `Looking for files… ${plural(scan.filesFound, "file")} found`;
    case "hashing":
      return `Checking ${formatCount(scan.filesProcessed)} of ${plural(scan.filesFound, "file")}`;
    case "reading":
      return `Reading ${formatCount(scan.filesRead)} of ${plural(scan.filesToRead, "document")}`;
    case "embedding":
      return `Understanding ${formatCount(scan.passagesEmbedded)} of ${plural(scan.passagesToEmbed, "passage")}`;
    case "done": {
      const problems = scan.filesFailed + scan.readFailed;
      const read = scan.filesToRead > 0 ? ` · ${formatCount(scan.filesToRead - scan.readFailed)} read` : "";
      const failed = problems > 0 ? ` · ${formatCount(problems)} couldn't be read` : "";
      return `${plural(scan.filesFound, "file")} found${read}${failed}`;
    }
    case "failed":
      return scan.error ?? "Scanning failed.";
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
  fileCount?: number;
  className?: string;
}

export function ScanProgress({ scan, fileCount, className }: ScanProgressProps) {
  const percent = scanPercent(scan);

  return (
    <div className={cn("space-y-1.5", className)}>
      <p className={cn("text-xs", scan?.state === "failed" ? "text-destructive" : "text-muted-foreground")}>
        {describeScan(scan, fileCount)}
      </p>
      {isScanning(scan) && <Progress value={percent} label="Scan progress" />}
    </div>
  );
}
