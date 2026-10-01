import { useEffect, useState } from "react";

import { api, type ApiError, type AppInfo } from "@/lib/api/client";

/** Loads basic app info from the backend. Used to show backend connection status. */
export function useAppInfo() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [error, setError] = useState<ApiError | null>(null);

  useEffect(() => {
    let cancelled = false;
    api
      .getAppInfo()
      .then((result) => !cancelled && setInfo(result))
      .catch((err: ApiError) => !cancelled && setError(err));
    return () => {
      cancelled = true;
    };
  }, []);

  return { info, error, loading: !info && !error };
}
