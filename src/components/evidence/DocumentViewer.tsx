import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { ErrorMessage } from "@/components/ui/error-message";
import { useDocument } from "@/hooks/useDocument";
import { errorMessage, type IndexedFile } from "@/lib/api/client";
import { formatCount, plural } from "@/lib/format";

interface DocumentViewerProps {
  file: IndexedFile | null;
  onClose: () => void;
}

/** Shows exactly the text Recall extracted from a file, page by page. */
export function DocumentViewer({ file, onClose }: DocumentViewerProps) {
  const document = useDocument(file?.id ?? null);

  const details = document.data
    ? [
        document.data.pageCount !== null && plural(document.data.pageCount, "page"),
        plural(document.data.wordCount, "word"),
        plural(document.data.chunkCount, "searchable passage"),
        document.data.author && `by ${document.data.author}`,
      ].filter(Boolean)
    : [];

  return (
    <Dialog open={file !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{document.data?.title ?? file?.name}</DialogTitle>
          <DialogDescription className="break-all">{file?.path}</DialogDescription>
          {details.length > 0 && <p className="text-xs text-muted-foreground">{details.join(" · ")}</p>}
        </DialogHeader>

        <div className="-mx-6 min-h-0 flex-1 overflow-y-auto border-t px-6 pt-4">
          {document.isPending ? (
            <p className="text-sm text-muted-foreground">Loading…</p>
          ) : document.error ? (
            <ErrorMessage message={errorMessage(document.error)} />
          ) : !document.data ? (
            <p className="text-sm text-muted-foreground">
              {file?.error ?? "Recall hasn't read this file's text yet."}
            </p>
          ) : (
            <div className="flex flex-col gap-6 pb-2">
              {document.data.pages.map((page, index) => (
                <section key={index} aria-label={page.number ? `Page ${page.number}` : "Text"}>
                  {page.number !== null && (
                    <h3 className="mb-2 text-xs font-medium tracking-wide text-muted-foreground uppercase">
                      Page {formatCount(page.number)}
                    </h3>
                  )}
                  <p className="text-sm leading-relaxed whitespace-pre-wrap">{page.text}</p>
                </section>
              ))}
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
