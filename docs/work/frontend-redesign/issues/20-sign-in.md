# 20 — Sign-in

**What to build:** sign-in with GitHub in the UI, ahead of the community
screens (maintainer's ask, 2026-11-18): split out of 05, whose account
items it closes. The server half is `server-auth-github-oauth`; this is
contract §4's client.

**Blocked by:** 09.

**Status:** done

- [x] Sign in: the menu (and a top-bar button on desktop) stores a random
      `state` in `sessionStorage` and leaves for `/api/auth/github/login`
- [x] `/auth/callback` fails closed: it exchanges the code only when a
      stored `state` exists and equals GitHub's, and clears it either way;
      GitHub's own `error=` (the user cancelled) is reported, not retried
- [x] The session: refresh token in `localStorage`, access token in memory,
      one shared refresh at a time (tokens rotate), `/me` checked on load
- [x] Signed in: the avatar is the menu trigger; the menu shows the login
      and role, Sign out, and Delete account behind a dialog that describes
      identity removal (published work stays)
- [x] Strings in all nine locales

## Comments

**2026-11-18 — built.** `stores/session.ts` owns it; `authFetch` adds the
bearer token and refreshes once on a 401. GitHub returns to the *path*
`/auth/callback`, and the app routes by hash: `app/oauthCallback.ts`, the
first import in `main.ts`, moves the query into `#/auth/callback` before
the hash history reads the URL at import time (it did not, at first —
the router had already settled on `/`). Sign out forgets this device's
tokens only: the API has no revoke route, so the refresh token stays valid
server-side until it expires, held nowhere. Tested in a browser against a
faked GitHub and auth API: the round trip back to the starting page, a
reload restoring the session through one refresh, the callback with no
stored state refused, and deletion. Not tested against real GitHub: that
needs the OAuth app (production-v2's pre-deploy list).
