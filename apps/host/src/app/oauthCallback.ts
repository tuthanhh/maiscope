// GitHub returns to the registered callback, a *path*
// (`/auth/callback?code=…&state=…`); the app routes by hash. Move the callback
// into the hash so the `auth-callback` route picks it up — and the code
// leaves the path. This must run before `app/router.ts` is evaluated: the
// hash history reads the URL when it is created, at import time. main.ts
// imports this module first for that reason; keep it first.
if (window.location.pathname === "/auth/callback") {
  window.history.replaceState(null, "", `/#/auth/callback${window.location.search}`);
}
