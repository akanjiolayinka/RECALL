import { useQuery } from "@tanstack/react-query";

import { api } from "@/lib/api/client";

/** Loads basic app info from the backend. Used to show backend connection status. */
export function useAppInfo() {
  return useQuery({
    queryKey: ["appInfo"],
    queryFn: () => api.getAppInfo(),
    staleTime: Infinity,
  });
}
