import { useEffect, useState } from "react";
import { Search, TriangleAlert } from "lucide-react";

import { DocumentViewer } from "@/components/evidence/DocumentViewer";
import { Button } from "@/components/ui/button";
import { ErrorMessage } from "@/components/ui/error-message";
import { Input } from "@/components/ui/input";
import { useFiles } from "@/hooks/useFiles";
import { errorMessage, type FileKind, type IndexedFile } from "@/lib/api/client";
import { formatBytes, formatCount, formatDate } from "@/lib/format";
import { cn } from "@/lib/utils";

import { FILE_KIND_LABELS, FileKindIcon } from "./FileKindIcon";
import { FileStatusLabel } from "./FileStatusLabel";

const KIND_FILTERS: (FileKind | undefined)[] = [undefined, "pdf", "docx", "text", "markdown", "image"];
const PAGE_SIZE = 200;
const FILTER_DELAY_MS = 200;

/** Browse and filter the files Recall found, by name and type. */
export function FileBrowser() {
  const [kind, setKind] = useState<FileKind | undefined>();
  const [text, setText] = useState("");
  const [nameContains, setNameContains] = useState("");
  const [openFile, setOpenFile] = useState<IndexedFile | null>(null);

  // Wait until the user pauses typing before asking the backend.
  useEffect(() => {
    const timer = setTimeout(() => setNameContains(text), FILTER_DELAY_MS);
    return () => clearTimeout(timer);
  }, [text]);

  const files = useFiles({ kind, nameContains: nameContains || undefined, limit: PAGE_SIZE });

  return (
    <section className="flex flex-col gap-3" aria-label="Files">
      <div className="flex flex-wrap items-center gap-3">
        <h2 className="mr-auto text-lg font-semibold">Files</h2>
        <div className="relative w-64">
          <Search
            className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground"
            aria-hidden
          />
          <Input
            value={text}
            onChange={(e) => setText(e.target.value)}
            placeholder="Filter by file name"
            aria-label="Filter files by name"
            className="pl-9"
          />
        </div>
      </div>

      <div className="flex flex-wrap gap-1.5" role="group" aria-label="Filter by type">
        {KIND_FILTERS.map((option) => (
          <Button
            key={option ?? "all"}
            size="sm"
            variant={kind === option ? "secondary" : "ghost"}
            aria-pressed={kind === option}
            onClick={() => setKind(option)}
          >
            {option ? FILE_KIND_LABELS[option] : "All"}
          </Button>
        ))}
      </div>

      {files.error ? (
        <ErrorMessage message={errorMessage(files.error)} />
      ) : !files.data ? (
        <p className="text-sm text-muted-foreground">Loading…</p>
      ) : files.data.total === 0 ? (
        <p className="rounded-xl border border-dashed px-4 py-8 text-center text-sm text-muted-foreground">
          {kind || nameContains ? "No files match these filters." : "No supported files found yet."}
        </p>
      ) : (
        <>
          <div className={cn("overflow-hidden rounded-xl border bg-card", files.isPlaceholderData && "opacity-60")}>
            <table className="w-full table-fixed text-sm">
              <thead className="border-b bg-muted/40 text-left text-xs text-muted-foreground">
                <tr>
                  <th className="px-4 py-2 font-medium">Name</th>
                  <th className="w-24 px-4 py-2 font-medium">Type</th>
                  <th className="w-32 px-4 py-2 font-medium">Status</th>
                  <th className="w-28 px-4 py-2 text-right font-medium">Size</th>
                  <th className="w-32 px-4 py-2 font-medium">Modified</th>
                </tr>
              </thead>
              <tbody className="divide-y">
                {files.data.files.map((file) => (
                  <tr key={file.id} className="hover:bg-muted/40">
                    <td className="px-4 py-2">
                      <div className="flex items-center gap-2.5">
                        <FileKindIcon kind={file.kind} className="size-4 shrink-0 text-muted-foreground" />
                        <div className="min-w-0">
                          <button
                            type="button"
                            onClick={() => setOpenFile(file)}
                            className="block max-w-full truncate text-left outline-none hover:underline focus-visible:underline"
                          >
                            {file.name}
                          </button>
                          <p className="truncate text-xs text-muted-foreground" title={file.path}>
                            {file.path}
                          </p>
                          {file.error && (
                            <p className="flex items-center gap-1 text-xs text-destructive">
                              <TriangleAlert className="size-3" aria-hidden />
                              {file.error}
                            </p>
                          )}
                        </div>
                      </div>
                    </td>
                    <td className="px-4 py-2 text-muted-foreground">{FILE_KIND_LABELS[file.kind]}</td>
                    <td className="px-4 py-2 text-xs">
                      <FileStatusLabel file={file} />
                    </td>
                    <td className="px-4 py-2 text-right whitespace-nowrap text-muted-foreground tabular-nums">
                      {formatBytes(file.sizeBytes)}
                    </td>
                    <td className="px-4 py-2 text-muted-foreground">{formatDate(file.modifiedAt)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <p className="text-xs text-muted-foreground">
            {files.data.total > files.data.files.length
              ? `Showing the ${formatCount(files.data.files.length)} most recently modified of ${formatCount(files.data.total)} files. Use the filters to narrow it down.`
              : `${formatCount(files.data.total)} ${files.data.total === 1 ? "file" : "files"}`}
          </p>
        </>
      )}
      <DocumentViewer file={openFile} onClose={() => setOpenFile(null)} />
    </section>
  );
}
