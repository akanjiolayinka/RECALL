import { useQuery } from "@tanstack/react-query";

import { api } from "@/lib/api/client";

/** Which local AI features (meaning search, OCR) are available. Fixed for a run of the app. */
export function useAiStatus() {
  return useQuery({ queryKey: ["aiStatus"], queryFn: () => api.getAiStatus(), staleTime: Infinity });
}
