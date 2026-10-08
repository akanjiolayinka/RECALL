import { FolderPlus, Info } from "lucide-react";

import { PageHeader } from "@/components/layout/PageHeader";
import { FileBrowser } from "@/components/library/FileBrowser";
import { LocationList } from "@/components/library/LocationList";
import { Button } from "@/components/ui/button";
import { EmptyState } from "@/components/ui/empty-state";
import { ErrorMessage } from "@/components/ui/error-message";
import {
  useAddLocation,
  useLocations,
  useRemoveLocation,
  useRescanLocation,
} from "@/hooks/useLocations";
import { errorMessage } from "@/lib/api/client";

export function LibraryPage() {
  const locations = useLocations();
  const addLocation = useAddLocation();
  const removeLocation = useRemoveLocation();
  const rescanLocation = useRescanLocation();

  const addButton = (
    <Button onClick={() => addLocation.mutate()} disabled={addLocation.isPending}>
      <FolderPlus aria-hidden />
      {addLocation.isPending ? "Choosing…" : "Add folder"}
    </Button>
  );

  const replaced = addLocation.data?.replaced ?? [];
  const mutationError = addLocation.error ?? removeLocation.error ?? rescanLocation.error;

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-start justify-between gap-4">
        <PageHeader title="Library" description="The folders Recall will make searchable." />
        {locations.data && locations.data.length > 0 && addButton}
      </div>

      {mutationError && <ErrorMessage message={errorMessage(mutationError)} />}

      {replaced.length > 0 && (
        <div className="flex items-start gap-3 rounded-lg border bg-muted/50 p-3 text-sm" role="status">
          <Info className="mt-0.5 size-4 shrink-0 text-muted-foreground" aria-hidden />
          <p>
            {replaced.map((l) => l.name).join(", ")} {replaced.length === 1 ? "was" : "were"} already
            in your library and {replaced.length === 1 ? "is" : "are"} inside{" "}
            {addLocation.data?.location.name}, so {replaced.length === 1 ? "it was" : "they were"}{" "}
            merged into it.
          </p>
        </div>
      )}

      {locations.isPending ? (
        <p className="text-sm text-muted-foreground">Loading…</p>
      ) : locations.error ? (
        <ErrorMessage message={errorMessage(locations.error)} />
      ) : locations.data.length === 0 ? (
        <EmptyState
          icon={FolderPlus}
          title="No folders yet"
          description="Choose a folder with documents, notes or screenshots. Recall reads it on this computer and never uploads anything."
        >
          {addButton}
        </EmptyState>
      ) : (
        <>
          <LocationList
            locations={locations.data}
            onRemove={(location) => removeLocation.mutate(location.id)}
            onRescan={(location) => rescanLocation.mutate(location.id)}
            removingId={removeLocation.isPending ? removeLocation.variables : undefined}
          />
          <p className="text-xs text-muted-foreground">
            Recall reads the text of PDF, Word, text and Markdown files on this computer. Images
            are found but not read yet (OCR arrives in Milestone 9). Click a file to see the text
            Recall extracted.
          </p>
          <FileBrowser />
        </>
      )}
    </div>
  );
}
