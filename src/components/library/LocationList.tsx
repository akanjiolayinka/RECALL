import { Folder, RotateCw, Trash2 } from "lucide-react";

import { isScanning, ScanProgress } from "@/components/indexing/ScanProgress";
import { Button } from "@/components/ui/button";
import type { Location } from "@/lib/api/client";

interface LocationListProps {
  locations: Location[];
  onRemove: (location: Location) => void;
  onRescan: (location: Location) => void;
  removingId?: string;
}

/** The folders in the user's library, with scan status and actions. */
export function LocationList({ locations, onRemove, onRescan, removingId }: LocationListProps) {
  return (
    <ul className="divide-y rounded-xl border bg-card">
      {locations.map((location) => (
        <li key={location.id} className="flex items-center gap-4 px-4 py-3">
          <div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
            <Folder className="size-4" aria-hidden />
          </div>
          <div className="min-w-0 flex-1 space-y-1">
            <div>
              <p className="truncate font-medium">{location.name}</p>
              <p className="truncate text-xs text-muted-foreground" title={location.path}>
                {location.path}
              </p>
            </div>
            <ScanProgress scan={location.scan} counts={location} className="max-w-sm" />
          </div>
          <div className="flex shrink-0 gap-1">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => onRescan(location)}
              disabled={isScanning(location.scan)}
              aria-label={`Check ${location.name} again for changes`}
            >
              <RotateCw aria-hidden />
              Rescan
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => onRemove(location)}
              disabled={removingId === location.id}
              aria-label={`Remove ${location.name} from library`}
            >
              <Trash2 aria-hidden />
              Remove
            </Button>
          </div>
        </li>
      ))}
    </ul>
  );
}
