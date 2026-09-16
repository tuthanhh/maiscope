# 03 — Server depends on `simai`; CI follows the crate

**What to build:** add `simai` to `apps/server`'s dependencies and move the
parser tests off the `engine` CI job.

**Blocked by:** 01 — extract `crates/simai`.

**Status:** todo

- [ ] `apps/server` depends on `simai` **without** the `bevy` feature
- [ ] `cargo tree -p server | grep bevy` is empty — the check that this whole
      feature exists for
- [ ] `ci.yml`: parser tests run in a job with no apt install and no Bevy cache.
      The `engine` job either shrinks to engine-only tests or disappears if it
      has none left
- [ ] The comment in `ci.yml` explaining why `rust` is scoped `-p server -p shared`
      gains a line about feature unification — a future `--workspace` step would
      pull Bevy back in through `simai/bevy`
- [ ] CLAUDE.md's Testing section points at `cargo test -p simai`
- [ ] `docs/architecture.md` gains the crate

> No server code calls the parser yet — that is
> [`community-charts` 02](../../community-charts/issues/02-upload-maidata.md).
> This ticket only proves the dependency is clean, which is cheaper to verify now
> than to discover later.
