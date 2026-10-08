import { BookOpen, ExternalLink } from "lucide-react";

import { FILE_KIND_LABELS, FileKindIcon } from "@/components/library/FileKindIcon";
import { Button } from "@/components/ui/button";
import { useOpenFile } from "@/hooks/useSearch";
import { errorMessage, type SearchResult } from "@/lib/api/client";
import { cn } from "@/lib/utils";

/** A plain-language label instead of a raw score. */
function relevanceLabel(relevance: number): string {
  if (relevance >= 1) return "Strong match";
  if (relevance >= 0.5) return "Good match";
  return "Partial match";
}

interface SearchResultCardProps {
  result: SearchResult;
  onViewText: (result: SearchResult) => void;
}

export function SearchResultCard({ result, onViewText }: SearchResultCardProps) {
  const openFile = useOpenFile();

  return (
    <article className="flex flex-col gap-3 rounded-xl border bg-card p-4">
      <header className="flex items-start gap-3">
        <FileKindIcon kind={result.fileKind} className="mt-0.5 size-5 shrink-0 text-muted-foreground" />
        <div className="min-w-0 flex-1">
          <h3 className="truncate font-medium">{result.fileName}</h3>
          <p className="truncate text-xs text-muted-foreground" title={result.filePath}>
            {FILE_KIND_LABELS[result.fileKind]}
            {result.page !== null && ` · Page ${result.page}`} · {result.filePath}
          </p>
        </div>
        <span
          className={cn(
            "shrink-0 rounded-full px-2 py-0.5 text-xs font-medium",
            result.relevance >= 1 ? "bg-primary/10 text-primary" : "bg-muted text-muted-foreground",
          )}
        >
          {relevanceLabel(result.relevance)}
        </span>
      </header>

      <blockquote className="border-l-2 pl-3 text-sm leading-relaxed text-foreground/90">
        {result.snippet.map((part, index) =>
          part.highlight ? (
            <mark key={index} className="rounded-sm bg-amber-200/70 px-0.5 text-foreground dark:bg-amber-400/30">
              {part.text}
            </mark>
          ) : (
            <span key={index}>{part.text}</span>
          ),
        )}
      </blockquote>

      <footer className="flex flex-wrap items-center gap-2">
        {result.matchReasons.map((reason) => (
          <span key={reason} className="text-xs text-muted-foreground">
            {reason}
          </span>
        ))}
        <div className="ml-auto flex gap-1">
          <Button variant="ghost" size="sm" onClick={() => onViewText(result)}>
            <BookOpen aria-hidden />
            View text
          </Button>
          <Button variant="outline" size="sm" onClick={() => openFile.mutate(result.fileId)} disabled={openFile.isPending}>
            <ExternalLink aria-hidden />
            Open
          </Button>
        </div>
      </footer>
      {openFile.error && <p className="text-xs text-destructive">{errorMessage(openFile.error)}</p>}
    </article>
  );
}
