import type { ApiError } from "./types";

/** User-facing message for anything thrown by the API client. */
export function errorMessage(error: unknown): string {
  if (error && typeof error === "object" && "message" in error) {
    return String((error as ApiError).message);
  }
  return "Something went wrong.";
}
