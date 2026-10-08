import { CircleCheck, Clock, TriangleAlert } from "lucide-react";

import type { IndexedFile } from "@/lib/api/client";

/** Short label for where a file is in Recall's pipeline. */
export function FileStatusLabel({ file }: { file: IndexedFile }) {
  if (file.status === "indexed") {
    return (
      <span className="flex items-center gap-1.5 text-emerald-700 dark:text-emerald-400">
        <CircleCheck className="size-3.5" aria-hidden />
        Read
      </span>
    );
  }
  if (file.status === "error") {
    return (
      <span className="flex items-center gap-1.5 text-destructive" title={file.error ?? undefined}>
        <TriangleAlert className="size-3.5" aria-hidden />
        Couldn't read
      </span>
    );
  }
  return (
    <span className="flex items-center gap-1.5 text-muted-foreground">
      <Clock className="size-3.5" aria-hidden />
      {file.kind === "image" ? "Needs OCR" : "Waiting"}
    </span>
  );
}
