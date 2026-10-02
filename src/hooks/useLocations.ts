import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { api } from "@/lib/api/client";

const LOCATIONS_KEY = ["locations"];

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
    onSuccess: () => queryClient.invalidateQueries({ queryKey: LOCATIONS_KEY }),
  });
}
