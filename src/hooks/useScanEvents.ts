import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";

import { api, type Location } from "@/lib/api/client";

import { FILES_KEY } from "./useFiles";
import { LOCATIONS_KEY } from "./useLocations";

/**
 * Keeps cached data in sync with live scan progress from the backend.
 * Mount once, near the root of the app.
 */
export function useScanEvents() {
  const queryClient = useQueryClient();

  useEffect(() => {
    let unsubscribe: (() => void) | undefined;
    let cancelled = false;

    void api
      .onScanProgress((status) => {
        queryClient.setQueryData<Location[]>(LOCATIONS_KEY, (locations) =>
          locations?.map((l) => (l.id === status.locationId ? { ...l, scan: status } : l)),
        );
        if (status.state === "done" || status.state === "failed") {
          void queryClient.invalidateQueries({ queryKey: FILES_KEY });
        }
      })
      .then((stop) => {
        if (cancelled) stop();
        else unsubscribe = stop;
      });

    return () => {
      cancelled = true;
      unsubscribe?.();
    };
  }, [queryClient]);
}
