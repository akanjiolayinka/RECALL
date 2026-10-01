import { useState, type FormEvent } from "react";
import { Info, Search } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

const SUGGESTED_SEARCHES = [
  "Find my project budget",
  "Where did I save my apartment notes?",
  "Find the screenshot with the database architecture",
  "Find my headphone receipt",
];

export function SearchPage() {
  const [query, setQuery] = useState("");
  const [submitted, setSubmitted] = useState<string | null>(null);

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (query.trim()) setSubmitted(query.trim());
  }

  return (
    <div className="mx-auto flex w-full max-w-2xl flex-col gap-8 pt-16">
      <div className="space-y-2 text-center">
        <h1 className="text-3xl font-semibold tracking-tight">What are you trying to remember?</h1>
        <p className="text-sm text-muted-foreground">
          Describe it the way you remember it. Everything stays on this computer.
        </p>
      </div>

      <form onSubmit={handleSubmit} className="flex gap-2" role="search">
        <div className="relative flex-1">
          <Search
            className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground"
            aria-hidden
          />
          <Input
            autoFocus
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="e.g. the document about my project budget"
            aria-label="Search your files"
            className="h-11 pl-9"
          />
        </div>
        <Button type="submit" size="lg" className="h-11">
          Search
        </Button>
      </form>

      <div className="flex flex-wrap justify-center gap-2">
        {SUGGESTED_SEARCHES.map((suggestion) => (
          <Button
            key={suggestion}
            variant="outline"
            size="sm"
            className="rounded-full font-normal"
            onClick={() => setQuery(suggestion)}
          >
            {suggestion}
          </Button>
        ))}
      </div>

      {submitted && (
        <div className="flex items-start gap-3 rounded-lg border bg-muted/50 p-4 text-sm" role="status">
          <Info className="mt-0.5 size-4 shrink-0 text-muted-foreground" aria-hidden />
          <p>
            Search isn't built yet, so nothing was searched for “{submitted}”. It arrives once
            Recall can index your files (keyword search in Milestone 5, meaning-based search in
            Milestone 6).
          </p>
        </div>
      )}
    </div>
  );
}
