/**
 * Single place where the frontend talks to `apps/server`.
 *
 * Both callers previously did `await (await fetch(url)).json()`, which parses an
 * error body as if it were data. The server's error envelope is:
 *
 *   { "error": "database_error", "message": "internal error" }
 *
 * so a failed `/catalog` currently becomes a confusing crash inside
 * `preprocessData` rather than a legible "the server errored".
 *
 * Callers and what they do with a throw:
 *   - `stores/data.ts:loadData`  catches, sets `currentLoadingErrorMessage` from
 *     `err.message`, and renders LoadingStatus.ERROR. Whatever this throws is
 *     shown to the user.
 *   - `composables/useSheetSearch.ts:runSearch`  has only `finally`, so a throw
 *     propagates unhandled and leaves the previous results on screen.
 */

/** Error envelope returned by `apps/server` (`AppError::into_response`). */
type ApiErrorBody = { error?: string; message?: string };

/** Thrown for any non-2xx response. Carries the status so callers can branch. */
export class ApiError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

/**
 * Like `fetchJson`, but also hands back the response headers.
 *
 * Paginated endpoints carry their total in `X-Total-Count` rather than the body
 * (ADR-0015), and a caller cannot read that through `fetchJson`. Error handling
 * stays in one place: `fetchJson` delegates here.
 */
export async function fetchJsonWithHeaders<T>(
  url: string,
): Promise<{ data: T; headers: Headers }> {
  const response = await fetch(url);

  if (response.ok) {
    return { data: (await response.json()) as T, headers: response.headers };
  }

  // The body is a stream and can only be read once, so read it a single time
  // into a variable. `.catch` because a failure is not guaranteed to be JSON:
  // a Fly 502 or a cold-start timeout returns HTML or nothing, and letting
  // `json()` reject here would replace the real failure with a parse error.
  const body: ApiErrorBody = await response.json().catch(() => ({}));

  // `stores/data.ts` renders this verbatim, so prefer the server's own message
  // and fall back to something legible rather than an empty string.
  throw new ApiError(
    response.status,
    body.message ?? response.statusText ?? `request failed (${response.status})`,
  );
}

export async function fetchJson<T>(url: string): Promise<T> {
  return (await fetchJsonWithHeaders<T>(url)).data;
}
