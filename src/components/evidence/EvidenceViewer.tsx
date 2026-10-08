import { useEffect, useRef } from "react";
import { ExternalLink, SearchX } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { EmptyState } from "@/components/ui/empty-state";
import { ErrorMessage } from "@/components/ui/error-message";
import { useEvidence, useOpenFile } from "@/hooks/useSearch";
import { errorMessage, type SearchResult } from "@/lib/api/client";
import { highlightWords } from "@/lib/highlight";

interface EvidenceViewerProps {
  /** The result whose evidence to show, or null when closed. */
  result: SearchResult | null;
  onClose: () => void;
}

/**
 * Shows the stored text that supports a search result: the matching passage
 * highlighted inside its page. "The AI finds it. The source proves it."
 */
export function EvidenceViewer({ result, onClose }: EvidenceViewerProps) {
  const evidence = useEvidence(result?.passageId ?? null);
  const openFile = useOpenFile();
  const passageRef = useRef<HTMLElement>(null);

  // The words the search matched, to highlight inside the passage.
  const words = result?.snippet.filter((part) => part.highlight).map((part) => part.text) ?? [];

  useEffect(() => {
    passageRef.current?.scrollIntoView({ block: "center" });
  }, [evidence.data]);

  return (
    <Dialog open={result !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{result?.fileName}</DialogTitle>
          <DialogDescription className="break-all">
            {evidence.data?.page != null && `Page ${evidence.data.page} · `}
            {result?.filePath}
          </DialogDescription>
        </DialogHeader>

        <div className="-mx-6 min-h-0 flex-1 overflow-y-auto border-t px-6 pt-4">
          {evidence.isPending ? (
            <p className="text-sm text-muted-foreground">Loading…</p>
          ) : evidence.error ? (
            <ErrorMessage message={errorMessage(evidence.error)} />
          ) : !evidence.data ? (
            <EmptyState
              icon={SearchX}
              title="We couldn't find enough evidence for this result."
              description="The file may have changed since it was indexed. Try searching again, or open the original file."
            />
          ) : (
            <p className="pb-2 text-sm leading-relaxed whitespace-pre-wrap text-muted-foreground">
              {evidence.data.before}
              <mark
                ref={passageRef}
                className="rounded bg-amber-100 px-0.5 text-foreground dark:bg-amber-400/20"
                aria-label="Matching passage"
              >
                {highlightWords(evidence.data.passage, words).map((part, index) =>
                  part.highlight ? (
                    <strong key={index} className="font-semibold underline decoration-amber-500 decoration-2">
                      {part.text}
                    </strong>
                  ) : (
                    <span key={index}>{part.text}</span>
                  ),
                )}
              </mark>
              {evidence.data.after}
            </p>
          )}
        </div>

        <div className="flex items-center justify-between gap-3 border-t pt-4">
          <p className="text-xs text-muted-foreground">
            Text as Recall indexed it. Open the original to see the file itself.
          </p>
          <Button
            variant="outline"
            size="sm"
            onClick={() => result && openFile.mutate(result.fileId)}
            disabled={!result || openFile.isPending}
          >
            <ExternalLink aria-hidden />
            Open original
          </Button>
        </div>
        {openFile.error && <p className="text-xs text-destructive">{errorMessage(openFile.error)}</p>}
      </DialogContent>
    </Dialog>
  );
}
