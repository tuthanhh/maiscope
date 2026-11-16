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
 * `fetch` that turns a non-2xx response into an `ApiError`. The one place
 * the error envelope is read; everything else layers on it.
 */
async function fetchOk(url: string, init?: RequestInit): Promise<Response> {
  const response = await fetch(url, init);

  if (response.ok) {
    return response;
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

/**
 * Like `fetchJson`, but also hands back the response headers.
 *
 * Paginated endpoints carry their total in `X-Total-Count` rather than the body
 * (ADR-0015), and a caller cannot read that through `fetchJson`.
 */
export async function fetchJsonWithHeaders<T>(
  url: string,
  init?: RequestInit,
): Promise<{ data: T; headers: Headers }> {
  const response = await fetchOk(url, init);
  return { data: (await response.json()) as T, headers: response.headers };
}

export async function fetchJson<T>(url: string, init?: RequestInit): Promise<T> {
  return (await fetchJsonWithHeaders<T>(url, init)).data;
}

/**
 * The body as text, unparsed. For the catalog, which is stored verbatim in
 * IndexedDB (`utils/catalogCache.ts`) and parsed once by the caller — parsing
 * here and re-serialising for the store would cost a second 3MB pass.
 */
export async function fetchText(url: string, init?: RequestInit): Promise<string> {
  return (await fetchOk(url, init)).text();
}
