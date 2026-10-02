import { QueryClient } from "@tanstack/react-query";

/**
 * Shared TanStack Query client. Data comes from a local backend, so there is
 * no need to refetch when the window regains focus or to retry failures.
 */
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: { refetchOnWindowFocus: false, retry: false },
  },
});
