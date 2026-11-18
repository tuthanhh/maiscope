import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { GAME } from "~/app/game";
import { ApiError, fetchJson, fetchNoContent } from "~/utils/api";

// The signed-in session (contract §4). The refresh token is the session: it
// lives in `localStorage` so a reload keeps you signed in, and rotates on
// every use. The access token (a 15-minute JWT) lives in memory only and is
// re-issued from the refresh token when it is missing or expired.

export type SessionUser = {
  id: string;
  login: string;
  avatarUrl: string | null;
  role: "user" | "moderator" | "admin";
};

type Tokens = { token: string; refreshToken: string };
type SignInResponse = Tokens & { user: SessionUser };

const REFRESH_KEY = "mv:refresh-token";
const STATE_KEY = "mv:oauth-state";
const RETURN_KEY = "mv:oauth-return";

// Storage can throw (private mode, blocked site data); a session that cannot
// be stored lasts the tab and nothing more.
function read(storage: () => Storage, key: string): string | null {
  try {
    return storage().getItem(key);
  } catch {
    return null;
  }
}
function write(storage: () => Storage, key: string, value: string | null): void {
  try {
    if (value === null) storage().removeItem(key);
    else storage().setItem(key, value);
  } catch {
    // See `read`.
  }
}
const local = () => localStorage;
const session = () => sessionStorage;

/** 32 URL-safe characters: within the API's 16–128 `state` rule. */
function randomState(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(24));
  return btoa(String.fromCharCode(...bytes))
    .replace(/\+/g, "-")
    .replace(/\//g, "_")
    .replace(/=+$/, "");
}

/** No auth request waits longer than this: a hung one would pin "Signing in…". */
const TIMEOUT_MS = 15_000;

function isTokens(value: unknown): value is Tokens {
  const v = value as Partial<Tokens> | null;
  return typeof v?.token === "string" && typeof v?.refreshToken === "string";
}

/**
 * Where to land after sign-in: an in-app path (one leading `/`), never the
 * callback route itself — landing there would replay a spent code.
 */
function safeReturn(path: string | null): string {
  if (!path || !path.startsWith("/") || path.startsWith("//")) return "/";
  if (path.startsWith("/auth/callback")) return "/";
  return path;
}

export class SignInError extends Error {
  constructor(readonly kind: "state" | "refused" | "unreachable") {
    super(kind);
  }
}

export const useSessionStore = defineStore("session", () => {
  const user = ref<SessionUser | null>(null);
  const accessToken = ref<string | null>(null);
  /** False until the stored session, if any, has been checked on load. */
  const ready = ref(false);
  const signedIn = computed(() => user.value !== null);

  // Every sign-in and every sign-out starts a new epoch. A response that
  // left under an older one — a refresh in flight when you signed out, the
  // boot-time `/me` racing a fresh sign-in — is discarded rather than
  // written over the session that replaced it.
  let epoch = 0;
  function forget(): void {
    epoch++;
    accessToken.value = null;
    user.value = null;
    write(local, REFRESH_KEY, null);
  }

  // One refresh at a time: refresh tokens rotate, so two concurrent refreshes
  // with one token would sign the second caller out (contract §4).
  // `refused` ends the session (401); `unavailable` (network, 429, 5xx,
  // timeout, a malformed body) keeps it for the next try.
  type Refresh = "ok" | "refused" | "unavailable";
  let refreshing: Promise<Refresh> | null = null;
  function refresh(): Promise<Refresh> {
    refreshing ??= (async (): Promise<Refresh> => {
      const refreshToken = read(local, REFRESH_KEY);
      if (!refreshToken) return "refused";
      const mine = epoch;
      try {
        const tokens = await fetchJson<Tokens>(`${GAME.apiBaseUrl}/auth/refresh`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ refreshToken }),
          signal: AbortSignal.timeout(TIMEOUT_MS),
        });
        if (mine !== epoch) return "refused";
        if (!isTokens(tokens)) return "unavailable";
        accessToken.value = tokens.token;
        write(local, REFRESH_KEY, tokens.refreshToken);
        return "ok";
      } catch (err) {
        if (err instanceof ApiError && err.status === 401) {
          if (mine === epoch) forget();
          return "refused";
        }
        return "unavailable";
      }
    })().finally(() => {
      refreshing = null;
    });
    return refreshing;
  }

  /**
   * `fetch` with the access token, refreshed once and retried when the API
   * says it has expired. Throws `ApiError` like `fetchJson`: a 401 only when
   * the session is refused, a 503 when it could not be checked.
   */
  async function authFetch<T>(
    url: string,
    init: RequestInit = {},
    fetcher: (url: string, init: RequestInit) => Promise<T> = fetchJson<T>,
  ): Promise<T> {
    const ensure = async () => {
      const result = await refresh();
      if (result === "refused") throw new ApiError(401, "not signed in");
      if (result === "unavailable") throw new ApiError(503, "session could not be checked");
    };
    if (!accessToken.value) await ensure();
    const send = () =>
      fetcher(url, {
        signal: AbortSignal.timeout(TIMEOUT_MS),
        ...init,
        headers: { ...init.headers, Authorization: `Bearer ${accessToken.value}` },
      });
    try {
      return await send();
    } catch (err) {
      if (!(err instanceof ApiError) || err.status !== 401) throw err;
      await ensure();
      return send();
    }
  }

  /**
   * On load: a stored session is checked against `/me`, which sees a deletion
   * at once. Only a refused session is forgotten; offline, it stays stored.
   */
  async function restore(): Promise<void> {
    const mine = epoch;
    try {
      if (read(local, REFRESH_KEY)) {
        const me = await authFetch<SessionUser>(`${GAME.apiBaseUrl}/auth/me`);
        if (mine === epoch) user.value = me;
      }
    } catch (err) {
      if (err instanceof ApiError && err.status === 401 && mine === epoch) forget();
    } finally {
      ready.value = true;
    }
  }

  /** Leave for GitHub's consent page; `returnTo` is where to land afterwards. */
  function signIn(returnTo: string): void {
    const state = randomState();
    write(session, STATE_KEY, state);
    write(session, RETURN_KEY, returnTo);
    window.location.assign(
      `${GAME.apiBaseUrl}/auth/github/login?state=${encodeURIComponent(state)}`,
    );
  }

  /** Clear a sign-in that will not complete (GitHub said cancel or error). */
  function abandonSignIn(): void {
    write(session, STATE_KEY, null);
    write(session, RETURN_KEY, null);
  }

  /**
   * GitHub came back with `code` and `state`. The state must be the one this
   * tab sent — the login-CSRF check (contract §4) — and is used once.
   * Returns where to land: an in-app path, never the callback itself.
   */
  async function completeSignIn(code: string, state: string): Promise<string> {
    const expected = read(session, STATE_KEY);
    const stored = read(session, RETURN_KEY);
    abandonSignIn();
    if (!expected || expected !== state) throw new SignInError("state");
    // This sign-in supersedes whatever the boot-time restore is doing.
    epoch++;
    const mine = epoch;
    let body: SignInResponse;
    try {
      body = await fetchJson<SignInResponse>(
        `${GAME.apiBaseUrl}/auth/github/callback?code=${encodeURIComponent(code)}`,
        { signal: AbortSignal.timeout(TIMEOUT_MS) },
      );
    } catch (err) {
      throw new SignInError(err instanceof ApiError && err.status === 400 ? "refused" : "unreachable");
    }
    if (!isTokens(body) || !body.user) throw new SignInError("unreachable");
    if (mine === epoch) {
      accessToken.value = body.token;
      write(local, REFRESH_KEY, body.refreshToken);
      user.value = body.user;
    }
    return safeReturn(stored);
  }

  /**
   * Forget this device's session. The API has no sign-out route: the refresh
   * token stays valid on the server until it expires (30 days) but is gone
   * from this browser, which is the only place it was kept.
   */
  function signOut(): void {
    forget();
  }

  /** Remove the account's identity (contract §4), then sign out. */
  async function deleteAccount(): Promise<void> {
    await authFetch(`${GAME.apiBaseUrl}/auth/me`, { method: "DELETE" }, fetchNoContent);
    forget();
  }

  return {
    user,
    ready,
    signedIn,
    restore,
    signIn,
    abandonSignIn,
    completeSignIn,
    signOut,
    deleteAccount,
    authFetch,
  };
});
