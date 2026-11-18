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

  function store(tokens: Tokens): void {
    accessToken.value = tokens.token;
    write(local, REFRESH_KEY, tokens.refreshToken);
  }
  function forget(): void {
    accessToken.value = null;
    user.value = null;
    write(local, REFRESH_KEY, null);
  }

  // One refresh at a time: refresh tokens rotate, so two concurrent refreshes
  // with one token would sign the second caller out (contract §4).
  let refreshing: Promise<boolean> | null = null;
  function refresh(): Promise<boolean> {
    refreshing ??= (async () => {
      const refreshToken = read(local, REFRESH_KEY);
      if (!refreshToken) return false;
      try {
        store(
          await fetchJson<Tokens>(`${GAME.apiBaseUrl}/auth/refresh`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ refreshToken }),
          }),
        );
        return true;
      } catch (err) {
        // A refused token ends the session; a network failure keeps it for
        // the next try.
        if (err instanceof ApiError && err.status === 401) forget();
        return false;
      }
    })().finally(() => {
      refreshing = null;
    });
    return refreshing;
  }

  /**
   * `fetch` with the access token, refreshed once and retried when the API
   * says it has expired. Throws `ApiError` like `fetchJson`.
   */
  async function authFetch<T>(
    url: string,
    init: RequestInit = {},
    fetcher: (url: string, init: RequestInit) => Promise<T> = fetchJson<T>,
  ): Promise<T> {
    if (!accessToken.value && !(await refresh())) {
      throw new ApiError(401, "not signed in");
    }
    const send = () =>
      fetcher(url, {
        ...init,
        headers: { ...init.headers, Authorization: `Bearer ${accessToken.value}` },
      });
    try {
      return await send();
    } catch (err) {
      if (!(err instanceof ApiError) || err.status !== 401 || !(await refresh())) throw err;
      return send();
    }
  }

  /** On load: a stored session is checked against `/me`, which sees a deletion at once. */
  async function restore(): Promise<void> {
    try {
      if (read(local, REFRESH_KEY)) {
        user.value = await authFetch<SessionUser>(`${GAME.apiBaseUrl}/auth/me`);
      }
    } catch (err) {
      if (err instanceof ApiError && err.status === 401) forget();
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

  /**
   * GitHub came back with `code` and `state`. The state must be the one this
   * tab sent — the login-CSRF check (contract §4) — and is used once.
   * Returns where to land.
   */
  async function completeSignIn(code: string, state: string): Promise<string> {
    const expected = read(session, STATE_KEY);
    const returnTo = read(session, RETURN_KEY) ?? "/";
    write(session, STATE_KEY, null);
    write(session, RETURN_KEY, null);
    if (!expected || expected !== state) throw new SignInError("state");
    try {
      const body = await fetchJson<SignInResponse>(
        `${GAME.apiBaseUrl}/auth/github/callback?code=${encodeURIComponent(code)}`,
      );
      store(body);
      user.value = body.user;
      return returnTo;
    } catch (err) {
      throw new SignInError(err instanceof ApiError && err.status === 400 ? "refused" : "unreachable");
    }
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
    completeSignIn,
    signOut,
    deleteAccount,
    authFetch,
  };
});
