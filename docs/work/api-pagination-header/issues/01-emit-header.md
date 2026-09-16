# 01 — Emit `X-Total-Count`, keep the body field

**What to build:** the expand step. `/sheets/search` sets an `X-Total-Count`
response header carrying the same number as `total`, and the CORS layer exposes
it. The body is unchanged, so nothing that reads `total` notices.

`search_sheets` currently returns `Json<SheetSearchResponse>`. It becomes an
`impl IntoResponse` that carries both the header and the body — the smallest
change that keeps the handler's signature honest about what it produces.

**Blocked by:** None.

**Status:** done

- [x] `search_sheets` sets `X-Total-Count` to the same value as the body `total`
- [x] `CorsLayer` gains `.expose_headers([HeaderName::from_static("x-total-count")])`
- [x] A test asserts the header is present and equals the body field — the
      invariant that makes step 03 safe to take later
- [x] A test through the assembled `Router` (`tower::ServiceExt::oneshot`, as
      `routes/mod.rs` does) asserts the CORS layer exposes it. A handler-level
      test cannot see the layer, which is exactly how this gets missed
- [x] `api-contract.md` §1.2 notes both are sent during the migration

> Deploy this before touching the frontend. It is additive, so the currently
> deployed bundle keeps working throughout.
