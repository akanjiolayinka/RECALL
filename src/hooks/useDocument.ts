import { useQuery } from "@tanstack/react-query";

import { api } from "@/lib/api/client";

/** The text Recall extracted from a file. Pass null to load nothing. */
export function useDocument(fileId: string | null) {
  return useQuery({
    queryKey: ["document", fileId],
    queryFn: () => api.getDocument(fileId!),
    enabled: fileId !== null,
  });
}
