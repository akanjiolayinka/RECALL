import { keepPreviousData, useMutation, useQuery } from "@tanstack/react-query";

import { api } from "@/lib/api/client";

/** Search results for `query`. Nothing is fetched for an empty query. */
export function useSearch(query: string) {
  const trimmed = query.trim();
  return useQuery({
    queryKey: ["search", trimmed],
    queryFn: () => api.search({ query: trimmed }),
    enabled: trimmed.length > 0,
    placeholderData: keepPreviousData,
  });
}

/** Opens a file in its default app. */
export function useOpenFile() {
  return useMutation({ mutationFn: (fileId: string) => api.openFile(fileId) });
}
