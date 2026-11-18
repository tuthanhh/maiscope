/**
 * The JSON and text fetches to `apps/server`. A non-2xx response becomes an
 * `ApiError` carrying the status and the server's message, rather than an
 * error envelope parsed as if it were data. The server's envelope is:
 *
 *   { "error": "database_error", "message": "internal error" }
 *
 * `stores/data.ts:loadData` shows the message to the user; the song modal
 * branches on the status (404 is "Song not found"). The chart text is the one
 * fetch outside this file: its errors are plain text (`useEngine.fetchChart`).
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
