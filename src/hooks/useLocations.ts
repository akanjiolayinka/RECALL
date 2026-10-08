import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { api } from "@/lib/api/client";

import { FILES_KEY } from "./useFiles";

export const LOCATIONS_KEY = ["locations"];

/** The folders the user has added to their library. */
export function useLocations() {
  return useQuery({ queryKey: LOCATIONS_KEY, queryFn: () => api.listLocations() });
}

/** Opens the system folder picker and adds the chosen folder. */
export function useAddLocation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => api.addLocation(),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: LOCATIONS_KEY }),
  });
}

export function useRemoveLocation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.removeLocation(id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: LOCATIONS_KEY });
      void queryClient.invalidateQueries({ queryKey: FILES_KEY });
    },
  });
}

/** Scans a folder again. Progress arrives through `useScanEvents`. */
export function useRescanLocation() {
  return useMutation({ mutationFn: (id: string) => api.rescanLocation(id) });
}
