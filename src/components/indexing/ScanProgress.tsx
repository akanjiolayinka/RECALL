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
    case "done": {
      const readable = scan.filesFound - scan.filesFailed;
      const problems = scan.filesFailed > 0 ? ` · ${formatCount(scan.filesFailed)} couldn't be read` : "";
      return `${plural(readable, "file")} found${problems}`;
    }
    case "failed":
      return scan.error ?? "Scanning failed.";
  }
}

export function isScanning(scan: ScanStatus | null): boolean {
  return scan?.state === "discovering" || scan?.state === "hashing";
}

/** Status line plus progress bar while a scan is running. */
interface ScanProgressProps {
  scan: ScanStatus | null;
  fileCount?: number;
  className?: string;
}

export function ScanProgress({ scan, fileCount, className }: ScanProgressProps) {
  const percent =
    scan?.state === "hashing" && scan.filesFound > 0
      ? (scan.filesProcessed / scan.filesFound) * 100
      : null;

  return (
    <div className={cn("space-y-1.5", className)}>
      <p className={cn("text-xs", scan?.state === "failed" ? "text-destructive" : "text-muted-foreground")}>
        {describeScan(scan, fileCount)}
      </p>
      {isScanning(scan) && <Progress value={percent} label="Scan progress" />}
    </div>
  );
}
