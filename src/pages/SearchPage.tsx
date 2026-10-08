import { useState, type FormEvent } from "react";
import { FolderPlus, Info, Search, SearchX } from "lucide-react";

import { DocumentViewer, type ViewerFile } from "@/components/evidence/DocumentViewer";
import { SearchResultCard } from "@/components/search/SearchResultCard";
import { Button } from "@/components/ui/button";
import { EmptyState } from "@/components/ui/empty-state";
import { ErrorMessage } from "@/components/ui/error-message";
import { Input } from "@/components/ui/input";
import { useLocations } from "@/hooks/useLocations";
import { useSearch, useSearchCapabilities } from "@/hooks/useSearch";
import { errorMessage } from "@/lib/api/client";
import { plural } from "@/lib/format";
import { cn } from "@/lib/utils";

const SUGGESTED_SEARCHES = [
  "Find my project budget",
  "Where did I save my apartment notes?",
  "Find the screenshot with the database architecture",
  "Find my headphone receipt",
];

export function SearchPage() {
  const [text, setText] = useState("");
  const [query, setQuery] = useState("");
  const [viewing, setViewing] = useState<ViewerFile | null>(null);
  const results = useSearch(query);
  const capabilities = useSearchCapabilities();
  const semanticOff = capabilities.data && !capabilities.data.semanticSearch;
  const locations = useLocations();
  const hasLibrary = (locations.data?.length ?? 0) > 0;

  function runSearch(value: string) {
    setText(value);
    setQuery(value.trim());
  }

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    runSearch(text);
  }

  return (
    <div className={cn("mx-auto flex w-full max-w-3xl flex-col gap-6 transition-[padding]", query ? "pt-4" : "pt-16")}>
      {!query && (
        <div className="space-y-2 text-center">
          <h1 className="text-3xl font-semibold tracking-tight">What are you trying to remember?</h1>
          <p className="text-sm text-muted-foreground">
            Describe it the way you remember it. Everything stays on this computer.
          </p>
        </div>
      )}

      <form onSubmit={handleSubmit} className="flex gap-2" role="search">
        <div className="relative flex-1">
          <Search
            className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground"
            aria-hidden
          />
          <Input
            autoFocus
            value={text}
            onChange={(e) => setText(e.target.value)}
            placeholder="e.g. the document about my project budget"
            aria-label="Search your files"
            className="h-11 pl-9"
          />
        </div>
        <Button type="submit" size="lg" className="h-11">
          Search
        </Button>
      </form>

      {!query && (
        <div className="flex flex-wrap justify-center gap-2">
          {SUGGESTED_SEARCHES.map((suggestion) => (
            <Button
              key={suggestion}
              variant="outline"
              size="sm"
              className="rounded-full font-normal"
              onClick={() => runSearch(suggestion)}
            >
              {suggestion}
            </Button>
          ))}
        </div>
      )}

      {semanticOff && capabilities.data?.semanticUnavailableReason && (
        <p className="flex items-start gap-2 text-xs text-muted-foreground">
          <Info className="mt-px size-3.5 shrink-0" aria-hidden />
          {capabilities.data.semanticUnavailableReason}
        </p>
      )}

      {locations.data && !hasLibrary ? (
        <EmptyState
          icon={FolderPlus}
          title="Add a folder to search"
          description="Recall only searches folders you've added in Library. Add one and Recall will read it on this computer."
        />
      ) : query && results.error ? (
        <ErrorMessage message={errorMessage(results.error)} />
      ) : query && results.data ? (
        results.data.length === 0 ? (
          <EmptyState
            icon={SearchX}
            title="No matches"
            description={
              semanticOff
                ? `No file names or passages contain the words in “${query}”. Without the local AI model Recall matches words, not meaning, so try different words.`
                : `Nothing in your indexed files matches “${query}”.`
            }
          />
        ) : (
          <section aria-label="Search results" className={cn("flex flex-col gap-3", results.isPlaceholderData && "opacity-60")}>
            <p className="text-xs text-muted-foreground">{plural(results.data.length, "matching file")}</p>
            {results.data.map((result) => (
              <SearchResultCard
                key={result.id}
                result={result}
                onViewText={(r) => setViewing({ id: r.fileId, name: r.fileName, path: r.filePath })}
              />
            ))}
          </section>
        )
      ) : query ? (
        <p className="text-sm text-muted-foreground">Searching…</p>
      ) : null}

      <DocumentViewer file={viewing} onClose={() => setViewing(null)} />
    </div>
  );
}
