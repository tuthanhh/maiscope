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

**2026-11-18 — review.** Applied: a network error, 429 or 5xx from the
refresh used to read as "signed out" and delete the stored session on any
offline page load — refresh now answers ok / refused / unavailable, and
only refused forgets; tested both ways. Every sign-in and sign-out starts
a new epoch, and a response from an older one is discarded, so a refresh
in flight cannot undo a sign-out, and the boot-time `/me` cannot overwrite
a fresh sign-in. The callback strips `code` and `state` from the URL
before the exchange; GitHub's `error=` clears the stored state too; the
landing path is an in-app path and never the callback itself. Auth
requests time out after 15s, and a malformed token body is not stored.
`public/_redirects` serves the app at `/auth/callback` on Cloudflare Pages
explicitly, rather than by its no-404.html default. The delete dialog has
its own failure message, closes if the session was refused meanwhile, and
returns focus to the account trigger; the avatar is fetched with
`referrerpolicy="no-referrer"`. Open: a Content-Security-Policy, the
remaining mitigation for a refresh token in `localStorage` — it must allow
the wasm engine, the cover CDN and the per-environment API origin, so it is
its own change. Accepted: the code reaches Pages' access log on GitHub's
first request (a redirect URI cannot carry a fragment); it is single-use
and short-lived.

**2026-11-18 — tests.** `stores/session.test.ts` (Vitest, the frontend's
first): restore with nothing stored, a good session, a refused one, and one
that cannot be checked (offline, 503, 429) or comes back malformed; one
refresh for concurrent calls; a sign-out that a refresh in flight cannot
undo; the callback refused without or with a mismatched state before any
request, signing in to the starting page, never landing on the callback or
off-origin, a refused code; a sign-in state the API accepts. Breaking the
epoch check or the landing-path check fails them.
