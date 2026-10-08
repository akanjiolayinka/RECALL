import { useQuery } from "@tanstack/react-query";

import { api } from "@/lib/api/client";

/** What Recall stores, measured from the running app. Refreshed each visit. */
export function usePrivacyReport() {
  return useQuery({ queryKey: ["privacyReport"], queryFn: () => api.getPrivacyReport(), staleTime: 0 });
}
