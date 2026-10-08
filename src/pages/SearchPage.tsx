import { useState, type FormEvent } from "react";
import { FolderPlus, Info, Search, SearchX } from "lucide-react";

import { EvidenceViewer } from "@/components/evidence/EvidenceViewer";
import { SearchResultCard } from "@/components/search/SearchResultCard";
import { Button } from "@/components/ui/button";
import { EmptyState } from "@/components/ui/empty-state";
import { ErrorMessage } from "@/components/ui/error-message";
import { Input } from "@/components/ui/input";
import { useLocations } from "@/hooks/useLocations";
import { useAiStatus } from "@/hooks/useAiStatus";
import { useSearch } from "@/hooks/useSearch";
import { errorMessage, type SearchResult } from "@/lib/api/client";
import { SEARCH_INPUT_ID } from "@/lib/shortcuts";
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
  const [evidenceFor, setEvidenceFor] = useState<SearchResult | null>(null);
  const results = useSearch(query);
  const aiStatus = useAiStatus();
  const semantic = aiStatus.data?.semanticSearch;
  const semanticOff = semantic !== undefined && !semantic.available;
  const locations = useLocations();
  const hasLibrary = (locations.data?.length ?? 0) > 0;

  function runSearch(value: string) {
    setText(value);
    // Searching the same words again should look again, not reuse old results.
    if (value.trim() === query) void results.refetch();
    else setQuery(value.trim());
  }

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    runSearch(text);
  }

  return (
    <div className={cn("mx-auto flex w-full max-w-3xl flex-col gap-6 transition-[padding]", query ? "pt-4" : "pt-16")}>
      {query && <h1 className="sr-only">Search results for “{query}”</h1>}
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
            id={SEARCH_INPUT_ID}
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

      {semanticOff && semantic.unavailableReason && (
        <p className="flex items-start gap-2 text-xs text-muted-foreground">
          <Info className="mt-px size-3.5 shrink-0" aria-hidden />
          {semantic.unavailableReason}
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
              <SearchResultCard key={result.id} result={result} onShowSource={setEvidenceFor} />
            ))}
          </section>
        )
      ) : query ? (
        <p className="text-sm text-muted-foreground">Searching…</p>
      ) : null}

      <EvidenceViewer result={evidenceFor} onClose={() => setEvidenceFor(null)} />
    </div>
  );
}
